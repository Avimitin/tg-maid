use reqwest::Url;
use serde::Deserialize;
use std::path::PathBuf;
use std::process::Stdio;
use tokio::process;

use crate::helper::Html;

#[derive(Deserialize, Debug)]
pub struct YtdlpVideo {
    pub uploader: String,
    pub uploader_id: String,
    pub description: String,
    pub fulltitle: String,
    pub webpage_url: String,
    pub webpage_url_domain: String,
    pub width: u32,
    pub height: u32,

    #[serde(skip)]
    pub video_filepath: PathBuf,
    #[serde(skip)]
    pub info_filepath: PathBuf,
    #[serde(skip)]
    pub thumbnail_filepath: PathBuf,
}

impl YtdlpVideo {
    pub async fn dl_from_url(url: &str) -> anyhow::Result<Self> {
        Url::parse(url).map_err(|og| anyhow::anyhow!("Invalid URL: {url}, parse error: {og}"))?;
        let result = process::Command::new("yt-dlp")
            .arg(url)
            .arg("--write-info-json")
            .arg("--write-thumbnail")
            .arg("--max-filesize")
            .arg("49.9M")
            .arg("--restrict-filenames")
            .arg("--no-progress")
            .arg("--print")
            .arg("after_move:filepath")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await?;
        if !result.status.success() {
            anyhow::bail!("{}", String::from_utf8_lossy(&result.stderr));
        }

        let filepath = String::from_utf8_lossy(&result.stdout).trim().to_string();
        if filepath.is_empty() {
            anyhow::bail!("Video too large");
        }
        let video_path = PathBuf::from(filepath);
        let Some(ext) = video_path.extension().map(|a| a.to_str().unwrap()) else {
            anyhow::bail!("Video too large");
        };
        let filename = video_path
            .file_name()
            .expect("[ytdlp] must have filename")
            .to_str()
            .expect("[ytdlp] must be UTF-8")
            .strip_suffix(&format!(".{ext}"))
            .expect("[ytdlp] must have extension");
        let dl_info_path = PathBuf::from(format!("{filename}.info.json"));
        let dl_info = tokio::fs::read(&dl_info_path).await?;

        let thumbnail = PathBuf::from(format!("{filename}.jpg"));

        let mut info_file: Self = serde_json::from_slice(&dl_info)?;
        info_file.video_filepath = video_path;
        info_file.info_filepath = dl_info_path;
        info_file.thumbnail_filepath = thumbnail;

        Ok(info_file)
    }

    pub fn as_tg_video_caption(&self) -> String {
        if self.webpage_url_domain == "bilibili.com" {
            let upload_profile_link = format!("https://space.bilibili.com/{ }", self.uploader_id);
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
        } else {
            format!("Unimplement platform")
        }
    }

    pub async fn clean(self) -> anyhow::Result<()> {
        tokio::fs::remove_file(self.video_filepath).await?;
        tokio::fs::remove_file(self.info_filepath).await?;
        tokio::fs::remove_file(self.thumbnail_filepath).await?;
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
