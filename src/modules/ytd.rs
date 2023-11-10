use anyhow::Context;
use serde::Deserialize;
use std::path::PathBuf;
use std::process::Stdio;
use tokio::process;
use walkdir::WalkDir;

use crate::helper::Html;

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
    pub filesize_approx: Option<u64>,
    pub thumbnail: String,

    #[serde(skip)]
    pub thumbnail_filepath: PathBuf,
    #[serde(skip)]
    pub maybe_playlist: bool,
}

const TELEGRAM_UPLOAD_LIMIT: u64 = 50;

impl YtdlpVideo {
    pub async fn dl_from_url(url: &str) -> anyhow::Result<Self> {
        let info = process::Command::new("yt-dlp")
            .arg(url)
            .arg("--restrict-filenames")
            .arg("-j")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await?;
        if !info.status.success() {
            anyhow::bail!("{}", String::from_utf8_lossy(&info.stderr))
        }

        let mut info: Self = serde_json::from_slice(&info.stdout)?;
        if let Some(true) = info.is_live {
            anyhow::bail!("Downloading livestream is not allowed");
        }
        if info.filesize_approx.is_none() {
            anyhow::bail!("Downloading livestream is not allowed");
        }

        let filesize = info.filesize_approx.unwrap();
        if (filesize / 1024 / 1024) > TELEGRAM_UPLOAD_LIMIT {
            anyhow::bail!(
                "Video too large, Telegram doesn't allow uploading video with file size larger than 50MB"
            );
        }

        let result = process::Command::new("yt-dlp")
            .arg(url)
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

#[tokio::test]
async fn test_download_video() {
    let info = YtdlpVideo::dl_from_url("https://www.bilibili.com/video/BV1JB4y1s7Dk/")
        .await
        .unwrap();
    dbg!(&info);
    info.clean().await.unwrap();
}
