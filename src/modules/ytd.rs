use crate::config::Config;
use crate::helper::Html;
use anyhow::Context;
use serde::Deserialize;
use std::path::PathBuf;
use std::process::Stdio;
use tokio::process;
use walkdir::WalkDir;

use super::video_dl::VideoDownloader;

#[derive(Deserialize, Debug)]
pub struct YtdlpVideo {
    pub id: String,
    pub uploader: String,
    pub uploader_id: String,
    pub description: String,
    pub fulltitle: String,
    pub webpage_url: String,
    pub webpage_url_domain: String,
    pub width: u32,
    pub height: u32,
    pub filename: String,
    pub is_live: Option<bool>,
    pub thumbnail: String,

    #[serde(skip)]
    pub thumbnail_filepath: PathBuf,
    #[serde(skip)]
    pub maybe_playlist: bool,
}

impl YtdlpVideo {
    pub async fn dl_from_url(url: &str) -> anyhow::Result<Self> {
        // Select video with mp4 format and size lower than 50M
        const QUALITY: [&str; 4] = ["b", "w", "b*", "w*"];
        const EXT: [&str; 2] = ["[ext=mp4]", ""];
        const SIZE: [&str; 2] = ["[filesize<50M]", "[filesize_approx<50M]"];
        let video_format = QUALITY
            .iter()
            .flat_map(move |qua| {
                EXT.iter().flat_map(move |ext| {
                    // Use the yt-dlp format to select video will returns video only & audio only
                    // format for BiliBili video.
                    //
                    // +wa means merge this video with the worst audio
                    SIZE.iter().map(move |size| format!("{qua}{ext}{size}+wa"))
                })
            })
            .collect::<Vec<_>>()
            .join("/");

        use which::which;
        let ytdlp = which("yt-dlp").expect("can not found yt-dlp program");
        let mut info = process::Command::new(ytdlp.to_str().unwrap());
        info.arg(url)
            .arg("--format")
            .arg(&video_format)
            .arg("--restrict-filenames")
            .arg("-j");
        if let Some(proxy_url) = Config::get_global_config().proxy.yt_dlp() {
            info.arg("--proxy").arg(proxy_url);
        }
        let info = info
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await?;
        if !info.status.success() {
            let err = String::from_utf8_lossy(&info.stderr);
            if err.contains("Requested format is not available") {
                anyhow::bail!(
                    "The requested video has no mp4 format (required for Telegram preview)\
                    or is larger than 50MB (Telegram max file limit for bot)"
                )
            }
            anyhow::bail!("{}", err)
        }

        let mut info: Self = serde_json::from_slice(&info.stdout)?;
        if let Some(true) = info.is_live {
            anyhow::bail!("Downloading livestream is not allowed");
        }

        let result = process::Command::new("yt-dlp")
            .arg(url)
            .arg("--format")
            .arg(&video_format)
            .arg("--write-thumbnail")
            .arg("--restrict-filenames")
            .arg("--no-progress")
            .arg("--no-playlist")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await?;
        if !result.status.success() {
            anyhow::bail!("{}", String::from_utf8_lossy(&result.stderr));
        }

        let video_path = PathBuf::from(&info.filename);
        match tokio::fs::try_exists(&video_path).await {
            Ok(true) => (),
            _ => {
                anyhow::bail!("No video file found, this might happen when yt-dlp download fail but exit with no error");
            }
        }
        let Some(ext) = video_path.extension().map(|a| a.to_str().unwrap()) else {
            anyhow::bail!("No extension found for this video, this should not be happened");
        };
        let filename = video_path
            .file_name()
            .expect("[ytdlp] must have filename")
            .to_str()
            .expect("[ytdlp] must be UTF-8")
            .strip_suffix(&format!(".{ext}"))
            .expect("[ytdlp] must have extension");

        let thumbnail = WalkDir::new(".")
            .into_iter()
            .filter_map(|p| p.ok())
            .filter(|p| p.path().extension().is_some())
            .filter(|p| {
                ["jpg", "png", "webp"].contains(&p.path().extension().unwrap().to_str().unwrap())
            })
            .find(|p| p.file_name().to_str().unwrap().contains(filename))
            .map(|x| x.path().to_path_buf());

        if thumbnail.is_none() {
            info.clean().await?;
            anyhow::bail!("No thumbnail for this video")
        }

        info.maybe_playlist = info.id.ends_with("_p1");
        info.thumbnail_filepath = thumbnail.unwrap();

        Ok(info)
    }

    pub fn as_tg_video_caption(&self) -> String {
        match self.webpage_url_domain.as_str() {
            "bilibili.com" => {
                let upload_profile_link =
                    format!("https://space.bilibili.com/{}", self.uploader_id);
                let uploader = Html::a(&upload_profile_link, &self.uploader);
                let video_title = Html::a(&self.webpage_url, &self.fulltitle);
                format!(
                    "视频：{}\n\
                上传者：{}\n\
                简介：{}...
                ",
                    video_title,
                    uploader,
                    self.description.chars().take(100).collect::<String>()
                )
            }
            "youtube.com" => {
                let upload_profile_link = format!("https://www.youtube.com/{}", self.uploader_id);
                let uploader = Html::a(&upload_profile_link, &self.uploader);
                let video_title = Html::a(&self.webpage_url, &self.fulltitle);
                format!(
                    "视频：{}\n\
                上传者：{}\n\
                简介：{}...
                ",
                    video_title,
                    uploader,
                    self.description.chars().take(100).collect::<String>()
                )
            }
            _ => "Unsupported platform".to_string(),
        }
    }

    pub async fn clean(self) -> anyhow::Result<()> {
        tokio::fs::remove_file(self.filename)
            .await
            .with_context(|| "fail to delete video")?;
        tokio::fs::remove_file(self.thumbnail_filepath)
            .await
            .with_context(|| "fail to delete thumbnail")?;
        Ok(())
    }
}

impl VideoDownloader for YtdlpVideo {
    async fn download_from_url(u: &str) -> anyhow::Result<Self> {
        Self::dl_from_url(u).await
    }

    fn provide_caption(&self) -> String {
        self.as_tg_video_caption()
    }
}

#[tokio::test]
async fn test_download_video() {
    let info = YtdlpVideo::dl_from_url("https://www.bilibili.com/video/BV1JB4y1s7Dk/")
        .await
        .unwrap();
    dbg!(&info);
    info.clean().await.unwrap();
}
