use crate::app_state::{App, Message, VideoState};
use iced::Task;
use image::RgbaImage;

/// Build a `file://` URL from a path, falling back gracefully instead of
/// panicking when the path cannot be expressed as a valid URL.
pub(crate) fn file_url_from_path(path: &std::path::Path) -> url::Url {
    url::Url::from_file_path(path).unwrap_or_else(|_| {
        let raw = path.display().to_string();
        url::Url::parse(&format!("file:///{}", raw)).unwrap_or_else(|e| {
            eprintln!("invalid file URL '{}': {}", raw, e);
            url::Url::parse("file:///").unwrap()
        })
    })
}

impl App {
    pub fn handle_subtitle_text(&mut self, text: String) -> Task<Message> {
        self.subtitle_text = crate::text_utils::clean_subtitle_text(&text);
        Task::none()
    }

    pub fn handle_subtitle_image(
        &mut self,
        img: Option<iced_video_player::pgs::PgsImage>,
    ) -> Task<Message> {
        self.subtitle_image = img.map(|i| {
            // Scale down PGS subtitle to 55% of original bitmap width
            // for a more comfortable reading size.
            let target_w = (i.width as f32 * 0.55_f32) as u32;
            if target_w >= i.width || i.width == 0 || i.height == 0 {
                return iced::widget::image::Handle::from_rgba(i.width, i.height, i.rgba);
            }
            let ratio = target_w as f32 / i.width as f32;
            let target_h = (i.height as f32 * ratio) as u32;
            let src = match RgbaImage::from_raw(i.width, i.height, i.rgba) {
                Some(img) => img,
                None => {
                    eprintln!(
                        "PGS RGBA size mismatch: {}x{} with {} bytes",
                        i.width,
                        i.height,
                        ((i.width as usize) * (i.height as usize) * 4)
                    );
                    return iced::widget::image::Handle::from_rgba(1, 1, vec![0u8, 0, 0, 0]);
                }
            };
            let scaled = image::imageops::resize(
                &src,
                target_w.max(1),
                target_h.max(1),
                image::imageops::FilterType::Lanczos3,
            );
            iced::widget::image::Handle::from_rgba(
                target_w.max(1),
                target_h.max(1),
                scaled.into_raw(),
            )
        });
        Task::none()
    }

    /// Subtitle selection priority:
    /// 1) explicit CLI `--subtitle`, 2) external English subtitle file,
    /// 3) extract the embedded English stream to an external SRT (async).
    pub(crate) fn apply_subtitle_auto(&mut self, video_path: &str) -> Task<Message> {
        if let Some(sp) = self.pending_subtitle.take() {
            self.load_subtitle_file(&sp);
            return Task::none();
        }

        if let Some(sub_path) = crate::subtitle_discovery::find_english_subtitle_file(video_path) {
            self.load_subtitle_file(&sub_path);
            return Task::none();
        }

        self.extract_embedded_english(video_path)
    }

    fn load_subtitle_file(&mut self, path: &std::path::Path) {
        if let Ok(sub_url) = url::Url::from_file_path(path)
            && let VideoState::Ready(ref mut vv) = self.video
            && let Err(e) = vv.set_subtitle_url(&sub_url)
        {
            eprintln!("Subtitle error: {}", e);
        }
    }

    fn extract_embedded_english(&mut self, video_path: &str) -> Task<Message> {
        let Some(english) = self.english_embedded_stream() else {
            return Task::none();
        };
        let (index, is_pgs) = (english.index, english.is_pgs);
        let path = video_path.to_string();
        Task::perform(
            async move { crate::subtitle_extract::extract_embedded_subtitle(&path, index, is_pgs) },
            Message::SubtitleExtracted,
        )
    }

    fn english_embedded_stream(&self) -> Option<iced_video_player::SubtitleStreamInfo> {
        let VideoState::Ready(ref v) = self.video else {
            return None;
        };
        let streams = v.subtitle_streams();
        streams
            .iter()
            .find(|s| s.english && s.is_text)
            .or_else(|| streams.iter().find(|s| s.english && s.is_pgs))
            .cloned()
    }

    pub fn handle_subtitle_extracted(
        &mut self,
        result: Result<std::path::PathBuf, String>,
    ) -> Task<Message> {
        match result {
            Ok(path) => {
                eprintln!("Subtitle extracted: {}", path.display());
                self.load_subtitle_file(&path);
            }
            Err(e) => eprintln!("Subtitle extraction failed: {}", e),
        }
        Task::none()
    }

    pub fn handle_load_subtitle(&mut self) -> Task<Message> {
        // Use AsyncFileDialog for the same reason as handle_open_file: the
        // synchronous dialog blocks the UI thread and causes the Windows
        // shell file enumeration to stall.
        Task::perform(
            async move {
                rfd::AsyncFileDialog::new()
                    .add_filter(
                        "Subtitle Files",
                        &["srt", "ass", "ssa", "vtt", "sub", "smi"],
                    )
                    .add_filter("All Files", &["*"])
                    .pick_file()
                    .await
                    .map(|handle| handle.path().to_path_buf())
            },
            Message::SubtitlePicked,
        )
    }

    pub fn handle_subtitle_picked(&mut self, path: Option<std::path::PathBuf>) -> Task<Message> {
        if let Some(path) = path {
            let url = file_url_from_path(&path);
            if let Some(Err(e)) = self.with_video_mut(|v| v.set_subtitle_url(&url)) {
                eprintln!("Failed to load subtitle: {}", e);
            }
        }
        Task::none()
    }
}
