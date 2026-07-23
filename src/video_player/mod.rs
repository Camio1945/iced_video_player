mod helpers;

use crate::{primitive::VideoPrimitive, video::Video};
use iced::{
    Element,
    advanced::{self, Widget, layout, widget},
    mouse,
};
use iced_wgpu::primitive::Renderer as PrimitiveRenderer;
use std::{marker::PhantomData, sync::atomic::Ordering, time::Duration};
use std::{sync::Arc, time::Instant};

/// Video player widget which displays the current frame of a [`Video`](crate::Video).
pub struct VideoPlayer<'a, Message, Theme = iced::Theme, Renderer = iced::Renderer>
where
    Renderer: PrimitiveRenderer,
{
    pub(crate) video: &'a Video,
    pub(crate) content_fit: iced::ContentFit,
    pub(crate) width: iced::Length,
    pub(crate) height: iced::Length,
    pub(crate) on_end_of_stream: Option<Message>,
    pub(crate) on_new_frame: Option<Message>,
    pub(crate) on_subtitle_text: Option<Box<dyn Fn(Option<String>) -> Message + 'a>>,
    pub(crate) on_subtitle_image: Option<Box<dyn Fn(Option<crate::pgs::PgsImage>) -> Message + 'a>>,
    pub(crate) on_error: Option<Box<dyn Fn(&glib::Error) -> Message + 'a>>,
    pub(crate) on_mouse_wheel_scrolled: Option<Box<dyn Fn(f64) -> Message + 'a>>,
    pub(crate) _phantom: PhantomData<(Theme, Renderer)>,
}

impl<'a, Message, Theme, Renderer> VideoPlayer<'a, Message, Theme, Renderer>
where
    Renderer: PrimitiveRenderer,
{
    /// Creates a new video player widget for a given video.
    pub fn new(video: &'a Video) -> Self {
        VideoPlayer {
            video,
            content_fit: iced::ContentFit::default(),
            width: iced::Length::Shrink,
            height: iced::Length::Shrink,
            on_end_of_stream: None,
            on_new_frame: None,
            on_subtitle_text: None,
            on_subtitle_image: None,
            on_error: None,
            on_mouse_wheel_scrolled: None,
            _phantom: Default::default(),
        }
    }

    /// Sets the width of the `VideoPlayer` boundaries.
    pub fn width(self, width: impl Into<iced::Length>) -> Self {
        VideoPlayer {
            width: width.into(),
            ..self
        }
    }

    /// Sets the height of the `VideoPlayer` boundaries.
    pub fn height(self, height: impl Into<iced::Length>) -> Self {
        VideoPlayer {
            height: height.into(),
            ..self
        }
    }

    /// Sets the `ContentFit` of the `VideoPlayer`.
    pub fn content_fit(self, content_fit: iced::ContentFit) -> Self {
        VideoPlayer {
            content_fit,
            ..self
        }
    }

    /// Message to send when the video reaches the end of stream (i.e., the video ends).
    pub fn on_end_of_stream(self, on_end_of_stream: Message) -> Self {
        VideoPlayer {
            on_end_of_stream: Some(on_end_of_stream),
            ..self
        }
    }

    /// Message to send when the video receives a new frame.
    pub fn on_new_frame(self, on_new_frame: Message) -> Self {
        VideoPlayer {
            on_new_frame: Some(on_new_frame),
            ..self
        }
    }

    /// Message to send when the video receives a new frame.
    pub fn on_subtitle_text<F>(self, on_subtitle_text: F) -> Self
    where
        F: 'a + Fn(Option<String>) -> Message,
    {
        VideoPlayer {
            on_subtitle_text: Some(Box::new(on_subtitle_text)),
            ..self
        }
    }

    /// Message to send when a bitmap subtitle (e.g. PGS) is decoded.
    /// `None` means the current subtitle should be cleared.
    pub fn on_subtitle_image<F>(self, on_subtitle_image: F) -> Self
    where
        F: 'a + Fn(Option<crate::pgs::PgsImage>) -> Message,
    {
        VideoPlayer {
            on_subtitle_image: Some(Box::new(on_subtitle_image)),
            ..self
        }
    }

    /// Message to send when the video playback encounters an error.
    pub fn on_error<F>(self, on_error: F) -> Self
    where
        F: 'a + Fn(&glib::Error) -> Message,
    {
        VideoPlayer {
            on_error: Some(Box::new(on_error)),
            ..self
        }
    }

    /// Message to send when the mouse wheel is scrolled over the video player.
    /// The `f64` parameter is the vertical scroll delta: positive for scroll up,
    /// negative for scroll down.
    pub fn on_mouse_wheel_scrolled<F>(self, on_mouse_wheel_scrolled: F) -> Self
    where
        F: 'a + Fn(f64) -> Message,
    {
        VideoPlayer {
            on_mouse_wheel_scrolled: Some(Box::new(on_mouse_wheel_scrolled)),
            ..self
        }
    }
}

impl<Message, Theme, Renderer> Widget<Message, Theme, Renderer>
    for VideoPlayer<'_, Message, Theme, Renderer>
where
    Message: Clone,
    Renderer: PrimitiveRenderer,
{
    fn size(&self) -> iced::Size<iced::Length> {
        iced::Size {
            width: iced::Length::Shrink,
            height: iced::Length::Shrink,
        }
    }

    fn layout(
        &mut self,
        _tree: &mut widget::Tree,
        _renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        let (video_width, video_height) = self.video.size();

        // based on `Image::layout`
        let image_size = iced::Size::new(video_width as f32, video_height as f32);
        let raw_size = limits.resolve(self.width, self.height, image_size);
        let full_size = self.content_fit.fit(image_size, raw_size);
        let final_size = iced::Size {
            width: match self.width {
                iced::Length::Shrink => f32::min(raw_size.width, full_size.width),
                _ => raw_size.width,
            },
            height: match self.height {
                iced::Length::Shrink => f32::min(raw_size.height, full_size.height),
                _ => raw_size.height,
            },
        };

        layout::Node::new(final_size)
    }

    fn draw(
        &self,
        _tree: &widget::Tree,
        renderer: &mut Renderer,
        _theme: &Theme,
        _style: &advanced::renderer::Style,
        layout: advanced::Layout<'_>,
        _cursor: advanced::mouse::Cursor,
        _viewport: &iced::Rectangle,
    ) {
        let mut inner = self.video.write();

        let image_size = iced::Size::new(inner.width as f32, inner.height as f32);
        let bounds = layout.bounds();
        let (drawing_bounds, adjusted_fit) =
            helpers::compute_video_drawing_bounds(image_size, bounds, self.content_fit);

        let upload_frame = inner.upload_frame.swap(false, Ordering::SeqCst);
        helpers::apply_av_offset_if_needed(&mut inner, upload_frame);

        let render = |renderer: &mut Renderer| {
            renderer.draw_primitive(
                drawing_bounds,
                VideoPrimitive::new(
                    inner.id,
                    Arc::clone(&inner.alive),
                    Arc::clone(&inner.frame),
                    (inner.width as _, inner.height as _),
                    upload_frame,
                ),
            );
        };

        if adjusted_fit.width > bounds.width || adjusted_fit.height > bounds.height {
            renderer.with_layer(bounds, render);
        } else {
            render(renderer);
        }
    }

    fn update(
        &mut self,
        _tree: &mut widget::Tree,
        event: &iced::Event,
        _layout: advanced::Layout<'_>,
        _cursor: advanced::mouse::Cursor,
        _renderer: &Renderer,
        _clipboard: &mut dyn advanced::Clipboard,
        shell: &mut advanced::Shell<'_, Message>,
        _viewport: &iced::Rectangle,
    ) {
        match event {
            iced::Event::Mouse(mouse::Event::WheelScrolled { delta }) => {
                if let Some(ref on_scroll) = self.on_mouse_wheel_scrolled {
                    let y_delta = match delta {
                        mouse::ScrollDelta::Lines { y, .. } => *y,
                        mouse::ScrollDelta::Pixels { y, .. } => *y / 4.0,
                    };
                    shell.publish(on_scroll(y_delta as f64));
                }
            }
            iced::Event::Window(iced::window::Event::RedrawRequested(_)) => {
                let mut inner = self.video.write();
                if inner.restart_stream || (!inner.is_eos && !inner.paused()) {
                    helpers::handle_active_redraw(&mut inner, self, shell);
                } else {
                    shell.request_redraw_at(iced::window::RedrawRequest::At(
                        Instant::now() + Duration::from_millis(32),
                    ));
                }
            }
            _ => {}
        }
    }
}

impl<'a, Message, Theme, Renderer> From<VideoPlayer<'a, Message, Theme, Renderer>>
    for Element<'a, Message, Theme, Renderer>
where
    Message: 'a + Clone,
    Theme: 'a,
    Renderer: 'a + PrimitiveRenderer,
{
    fn from(video_player: VideoPlayer<'a, Message, Theme, Renderer>) -> Self {
        Self::new(video_player)
    }
}
