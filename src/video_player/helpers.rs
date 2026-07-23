use crate::video_player::VideoPlayer;
use gstreamer as gst;
use iced::advanced::{self};
use iced_wgpu::primitive::Renderer as PrimitiveRenderer;
use log::error;
use std::sync::atomic::Ordering;
use std::time::Instant;

pub(crate) fn handle_active_redraw<Message: Clone, Theme, Renderer: PrimitiveRenderer>(
    inner: &mut crate::video::Internal,
    player: &VideoPlayer<'_, Message, Theme, Renderer>,
    shell: &mut advanced::Shell<'_, Message>,
) {
    let mut restart_stream = false;
    let emit_eos = !inner.restart_stream;
    if inner.restart_stream {
        restart_stream = true;
        inner.restart_stream = false;
    }
    let eos_pause = drain_gst_bus(
        inner,
        player.on_end_of_stream.clone(),
        player.on_error.as_ref(),
        emit_eos,
        &mut restart_stream,
        shell,
    );
    handle_stream_restart_or_pause(inner, restart_stream, eos_pause);
    notify_frame_and_subtitle(
        inner,
        player.on_new_frame.clone(),
        &player.on_subtitle_text,
        &player.on_subtitle_image,
        shell,
    );
    shell.request_redraw();
}

pub(crate) fn apply_av_offset_if_needed(inner: &mut crate::video::Internal, upload_frame: bool) {
    if upload_frame {
        let last_frame_time = inner
            .last_frame_time
            .lock()
            .map(|time| *time)
            .unwrap_or_else(|_| Instant::now());
        inner.set_av_offset(Instant::now() - last_frame_time);
    }
}

pub(crate) fn notify_frame_and_subtitle<Message: Clone>(
    inner: &mut crate::video::Internal,
    on_new_frame: Option<Message>,
    on_subtitle_text: &Option<Box<dyn Fn(Option<String>) -> Message + '_>>,
    on_subtitle_image: &Option<Box<dyn Fn(Option<crate::pgs::PgsImage>) -> Message + '_>>,
    shell: &mut advanced::Shell<'_, Message>,
) {
    if inner.upload_frame.load(Ordering::SeqCst)
        && let Some(on_new_frame) = on_new_frame
    {
        shell.publish(on_new_frame);
    }
    if let Some(on_subtitle_text) = on_subtitle_text
        && inner.upload_text.swap(false, Ordering::SeqCst)
        && let Ok(text) = inner.subtitle_text.try_lock()
    {
        shell.publish(on_subtitle_text(text.clone()));
    }
    if let Some(on_subtitle_image) = on_subtitle_image
        && inner.upload_image.swap(false, Ordering::SeqCst)
        && let Ok(img) = inner.subtitle_image.try_lock()
    {
        shell.publish(on_subtitle_image(img.clone()));
    }
}

pub(crate) fn compute_video_drawing_bounds(
    image_size: iced::Size,
    bounds: iced::Rectangle,
    content_fit: iced::ContentFit,
) -> (iced::Rectangle, iced::Size) {
    if image_size.width <= 0.0 || image_size.height <= 0.0 {
        return (bounds, bounds.size());
    }
    let adjusted_fit = content_fit.fit(image_size, bounds.size());
    let scale = iced::Vector::new(
        adjusted_fit.width / image_size.width,
        adjusted_fit.height / image_size.height,
    );
    let final_size = image_size * scale;

    let position = match content_fit {
        iced::ContentFit::None => iced::Point::new(
            bounds.x + (image_size.width - adjusted_fit.width) / 2.0,
            bounds.y + (image_size.height - adjusted_fit.height) / 2.0,
        ),
        _ => iced::Point::new(
            bounds.center_x() - final_size.width / 2.0,
            bounds.center_y() - final_size.height / 2.0,
        ),
    };

    (iced::Rectangle::new(position, final_size), adjusted_fit)
}

pub(crate) fn drain_gst_bus<Message: Clone>(
    inner: &mut crate::video::Internal,
    on_end_of_stream: Option<Message>,
    on_error: Option<&Box<dyn Fn(&glib::Error) -> Message + '_>>,
    emit_eos: bool,
    restart_stream: &mut bool,
    shell: &mut advanced::Shell<'_, Message>,
) -> bool {
    let mut eos_pause = false;

    while let Some(msg) = inner
        .bus
        .pop_filtered(&[gst::MessageType::Error, gst::MessageType::Eos])
    {
        match msg.view() {
            gst::MessageView::Error(err) => {
                error!("bus returned an error: {err}");
                if let Some(ref on_error) = on_error {
                    shell.publish(on_error(&err.error()))
                };
            }
            gst::MessageView::Eos(_eos) => {
                if emit_eos && let Some(ref on_eos) = on_end_of_stream {
                    shell.publish(on_eos.clone());
                }
                if inner.looping {
                    *restart_stream = true;
                } else {
                    eos_pause = true;
                }
            }
            _ => {}
        }
    }

    eos_pause
}

pub(crate) fn handle_stream_restart_or_pause(
    inner: &mut crate::video::Internal,
    restart_stream: bool,
    eos_pause: bool,
) {
    if restart_stream {
        if let Err(err) = inner.restart_stream() {
            error!("cannot restart stream (can't seek): {err:#?}");
        }
    } else if eos_pause {
        inner.is_eos = true;
        inner.set_paused(true);
    }
}
