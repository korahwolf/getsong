use A2VConverter::AudioVideoConverter;
use egui::Ui;
use std::{fs, path::PathBuf};
use tokio::sync::mpsc;
use yt_dlp::{Downloader, client::Libraries};

const YT_DLP_PATH: &str = "libs/yt-dlp";
const FFMPEG_PATH: &str = "libs/ffmpeg";

struct Getsong {
    urls: String,
    video_tx: mpsc::Sender<PathBuf>,
    video_rx: mpsc::Receiver<PathBuf>,
    downloading: bool,
    keep_video: bool,
    downloaded: Vec<PathBuf>,
}

impl Default for Getsong {
    fn default() -> Self {
        let (video_tx, video_rx) = mpsc::channel(1);

        Self {
            urls: String::new(),
            video_tx,
            video_rx,
            downloading: false,
            keep_video: false,
            downloaded: vec![],
        }
    }
}

impl Getsong {
    fn download(&mut self, video_url: String, ui: &mut Ui) {
        let libraries = Libraries::new(PathBuf::from(YT_DLP_PATH), PathBuf::from(FFMPEG_PATH));

        let video_tx = self.video_tx.clone();

        self.downloading = true;
        let ctx = ui.ctx().clone();

        tokio::spawn(async move {
            let out_dir = PathBuf::from("output");

            let downloader = match Downloader::builder(libraries, ".").build().await {
                Ok(d) => d,
                Err(_) => return,
            };

            let video = match downloader.fetch_video_infos(video_url).await {
                Ok(d) => d,
                Err(_) => return,
            };

            let out_path = format!("{}/{}.mp4", out_dir.display(), video.title);

            println!("downloading to: {}", out_path);

            downloader
                .download(&video, &out_path)
                .audio_quality(yt_dlp::model::AudioQuality::Best)
                .video_quality(yt_dlp::model::VideoQuality::Best)
                .execute()
                .await
                .expect("failed to download the video");

            println!("video fetched...");

            let new_path = PathBuf::from(out_path.replace(".mp4", ".mp3"));

            // do shit
            AudioVideoConverter::convert_video_to_audio(
                out_path.as_str(),
                new_path.to_str().unwrap(),
            )
            .unwrap();

            println!("video converted to audio and saved!");

            video_tx.send(new_path).await.ok();
            ctx.request_repaint();
        });
    }
}

impl eframe::App for Getsong {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        while let Ok(path) = self.video_rx.try_recv() {
            if !self.keep_video {
                fs::remove_file(path.to_string_lossy().replace(".mp3", ".mp4")).ok();
            }

            self.downloading = false;
            self.downloaded.push(path);
        }

        egui::CentralPanel::default().show(ui, |ui| {
            ui.heading("getsong");
            ui.separator();

            let mut current_url: String = "".to_owned();

            ui.label("enter as many urls as you want:");

            ui.horizontal(|ui| {
                ui.text_edit_multiline(&mut self.urls);
                ui.checkbox(&mut self.keep_video, "keep video");

                if ui.button("download").clicked() {
                    for url in self.urls.clone().lines() {
                        current_url = url.to_owned();
                        self.download(url.to_owned(), ui);
                    }
                }
            });

            if self.downloading {
                ui.label(format!("downloading {}...", current_url));
            }

            for path in &self.downloaded {
                ui.label(format!("{} downloaded", path.display()));
            }
        });
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let exe_dir = PathBuf::from("libs");
    let out_dir = PathBuf::from("output");

    Downloader::with_new_binaries(exe_dir, out_dir)
        .await?
        .build()
        .await?;

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([640.0, 360.0]),
        ..Default::default()
    };

    eframe::run_native(
        "getsong",
        options,
        Box::new(|cc| {
            egui_extras::install_image_loaders(&cc.egui_ctx);
            Ok(Box::<Getsong>::default())
        }),
    )?;

    Ok(())
}
