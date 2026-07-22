//! Minimal PGS (Presentation Graphic Stream / Blu-ray subtitle) decoder.
//!
//! Decodes a complete PGS display set (the contents of one GStreamer
//! `subpicture/x-pgs` buffer) into an RGBA bitmap with position info.

/// A decoded PGS subtitle image.
#[derive(Debug, Clone)]
pub struct PgsImage {
    pub rgba: Vec<u8>,
    pub width: u32,
    pub height: u32,
    /// X offset within the video frame.
    pub x: u32,
    /// Y offset within the video frame.
    pub y: u32,
    /// Video frame width the subtitle was authored for.
    pub frame_width: u32,
    /// Video frame height the subtitle was authored for.
    pub frame_height: u32,
}

struct OdsPart {
    object_id: u16,
    first: bool,
    last: bool,
    width: u16,
    height: u16,
    rle_data: Vec<u8>,
}

/// A display set parsed from a `.sup` stream, with its presentation time.
#[derive(Debug)]
pub struct PgsDisplaySet {
    /// Presentation timestamp in seconds (from the stream start).
    pub pts_seconds: f64,
    /// The decoded subtitle image, or `None` for a clear/empty set.
    pub image: Option<PgsImage>,
}

/// Parse a `.sup` stream (13-byte segment headers with PTS) into timestamped
/// display sets.  Only decodable content sets produce `Some(image)`; clear
/// sets are kept so callers can derive subtitle end times.
pub fn parse_sup(data: &[u8]) -> Vec<PgsDisplaySet> {
    let mut out = Vec::new();
    let mut pos = 0usize;
    let mut set_start = 0usize;
    let mut set_pts = 0u32;

    while pos + 13 <= data.len() {
        if data[pos] != 0x50 || data[pos + 1] != 0x47 {
            break;
        }
        let pts = u32::from_be_bytes([data[pos + 2], data[pos + 3], data[pos + 4], data[pos + 5]]);
        let seg_type = data[pos + 10];
        let seg_size = u16::from_be_bytes([data[pos + 11], data[pos + 12]]) as usize;
        if pos + 13 + seg_size > data.len() {
            break;
        }
        // A PCS segment anchors the start of a display set.
        if seg_type == 0x16 {
            set_start = pos;
            set_pts = pts;
        }
        pos += 13 + seg_size;
        if seg_type == 0x80 {
            out.push(PgsDisplaySet {
                pts_seconds: set_pts as f64 / 90000.0,
                image: decode(&data[set_start..pos]),
            });
        }
    }
    out
}

/// Try to decode a PGS display set from raw binary data.
/// Returns `None` if the data doesn't look like PGS or is incomplete.
///
/// Two segment framings are supported:
/// - `.sup` style: each segment has a 13-byte header (`"PG"` magic + PTS + DTS).
/// - GStreamer/matroskademux style: raw `[type][size][data]` triplets.
pub fn decode(data: &[u8]) -> Option<PgsImage> {
    let with_headers = data.len() >= 2 && data[0] == 0x50 && data[1] == 0x47;
    let header_size = if with_headers { 13 } else { 3 };

    let mut palette = [(0u8, 0u8, 0u8, 0u8); 256]; // (Y, Cr, Cb, A)
    let mut ods_parts: Vec<OdsPart> = Vec::new();
    let mut comp_w = 0u32;
    let mut comp_h = 0u32;
    let mut comp_objects: Vec<(u16, u16, u16)> = Vec::new(); // (obj_id, x, y)

    let mut pos = 0;
    while pos + header_size <= data.len() {
        let (seg_type, seg_size) = read_seg_header(data, pos, with_headers)?;
        if pos + header_size + seg_size > data.len() {
            break;
        }
        let seg = &data[pos + header_size..pos + header_size + seg_size];

        match seg_type {
            0x14 => parse_pds(seg, &mut palette),
            0x15 => {
                if let Some(part) = parse_ods(seg) {
                    ods_parts.push(part);
                }
            }
            0x16 => parse_pcs(seg, &mut comp_w, &mut comp_h, &mut comp_objects),
            0x80 => {
                return compose(&palette, &ods_parts, comp_w, comp_h, &comp_objects);
            }
            _ => {}
        }
        pos += header_size + seg_size;
    }
    None
}

fn read_seg_header(data: &[u8], pos: usize, with_headers: bool) -> Option<(u8, usize)> {
    if with_headers {
        if data[pos] != 0x50 || data[pos + 1] != 0x47 {
            return None;
        }
        let seg_type = data[pos + 10];
        let seg_size = u16::from_be_bytes([data[pos + 11], data[pos + 12]]) as usize;
        Some((seg_type, seg_size))
    } else {
        let seg_type = data[pos];
        let seg_size = u16::from_be_bytes([data[pos + 1], data[pos + 2]]) as usize;
        Some((seg_type, seg_size))
    }
}

/// Palette Definition Segment.
fn parse_pds(seg: &[u8], palette: &mut [(u8, u8, u8, u8); 256]) {
    // skip palette_id + version (2 bytes)
    let mut i = 2;
    while i + 5 <= seg.len() {
        let idx = seg[i] as usize;
        let y = seg[i + 1];
        let cr = seg[i + 2];
        let cb = seg[i + 3];
        let a = seg[i + 4];
        if idx < 256 {
            palette[idx] = (y, cr, cb, a);
        }
        i += 5;
    }
}

/// Object Definition Segment.
fn parse_ods(seg: &[u8]) -> Option<OdsPart> {
    if seg.len() < 7 {
        return None;
    }
    let object_id = u16::from_be_bytes([seg[0], seg[1]]);
    let seq = seg[3]; // 0x40=first, 0x80=last, 0xC0=only
    let data_len = ((seg[4] as usize) << 16) | ((seg[5] as usize) << 8) | seg[6] as usize;
    let header = if seq == 0x40 || seq == 0xC0 { 4 } else { 0 };
    if seg.len() < 7 + header {
        return None;
    }
    let (width, height) = if header == 4 {
        (
            u16::from_be_bytes([seg[7], seg[8]]),
            u16::from_be_bytes([seg[9], seg[10]]),
        )
    } else {
        (0, 0)
    };
    let rle_start = 7 + header;
    let rle_len = data_len.saturating_sub(header);
    let rle_end = (rle_start + rle_len).min(seg.len());
    Some(OdsPart {
        object_id,
        first: seq != 0x80,
        last: seq != 0x40,
        width,
        height,
        rle_data: seg[rle_start..rle_end].to_vec(),
    })
}

/// Presentation Composition Segment.
fn parse_pcs(seg: &[u8], w: &mut u32, h: &mut u32, objects: &mut Vec<(u16, u16, u16)>) {
    if seg.len() < 11 {
        return;
    }
    *w = u16::from_be_bytes([seg[0], seg[1]]) as u32;
    *h = u16::from_be_bytes([seg[2], seg[3]]) as u32;
    let num_objects = seg[10] as usize;
    objects.clear();
    let mut i = 11;
    for _ in 0..num_objects {
        if i + 8 > seg.len() {
            break;
        }
        let obj_id = u16::from_be_bytes([seg[i], seg[i + 1]]);
        let x = u16::from_be_bytes([seg[i + 4], seg[i + 5]]);
        let y = u16::from_be_bytes([seg[i + 6], seg[i + 7]]);
        objects.push((obj_id, x, y));
        i += 8;
    }
}

/// Assemble RLE parts into full bitmaps and compose the final image.
fn compose(
    palette: &[(u8, u8, u8, u8); 256],
    ods_parts: &[OdsPart],
    frame_w: u32,
    frame_h: u32,
    comp_objects: &[(u16, u16, u16)],
) -> Option<PgsImage> {
    if comp_objects.is_empty() || frame_w == 0 || frame_h == 0 {
        return None;
    }
    let full_objects = merge_ods_parts(ods_parts);

    // Use the first composition object.
    let &(obj_id, ox, oy) = comp_objects.first()?;
    let (_, ow, oh, rle) = full_objects.iter().find(|(id, ..)| *id == obj_id)?;
    let (w, h) = (*ow as usize, *oh as usize);
    if w == 0 || h == 0 {
        return None;
    }

    let indices = decode_rle(rle, w, h);
    let rgba = indices_to_rgba(palette, &indices);

    Some(PgsImage {
        rgba,
        width: w as u32,
        height: h as u32,
        x: ox as u32,
        y: oy as u32,
        frame_width: frame_w,
        frame_height: frame_h,
    })
}

/// Merge multi-part ODS segments into complete RLE streams per object.
fn merge_ods_parts(ods_parts: &[OdsPart]) -> Vec<(u16, u16, u16, Vec<u8>)> {
    let mut full: Vec<(u16, u16, u16, Vec<u8>)> = Vec::new();
    let mut current: Option<(u16, u16, u16, Vec<u8>)> = None;
    for part in ods_parts {
        if part.first {
            if let Some(obj) = current.take() {
                full.push(obj);
            }
            current = Some((
                part.object_id,
                part.width,
                part.height,
                part.rle_data.clone(),
            ));
        } else if let Some(ref mut cur) = current {
            cur.3.extend_from_slice(&part.rle_data);
        }
        if part.last
            && let Some(obj) = current.take()
        {
            full.push(obj);
        }
    }
    if let Some(obj) = current.take() {
        full.push(obj);
    }
    full
}

/// Convert an indexed-color bitmap through the YUV palette to RGBA.
fn indices_to_rgba(palette: &[(u8, u8, u8, u8); 256], indices: &[u8]) -> Vec<u8> {
    let mut rgba = Vec::with_capacity(indices.len() * 4);
    for &idx in indices {
        let (y, cr, cb, a) = palette[idx as usize];
        let (r, g, b) = yuv_to_rgb(y, cr, cb);
        rgba.extend_from_slice(&[r, g, b, a]);
    }
    rgba
}

/// Decode PGS RLE data into an indexed-color bitmap.
fn decode_rle(data: &[u8], width: usize, height: usize) -> Vec<u8> {
    let mut pixels = vec![0u8; width * height];
    let mut cur = RleCursor { x: 0, y: 0, pos: 0 };

    while cur.pos < data.len() && cur.y < height {
        let b = data[cur.pos];
        if b != 0 {
            if cur.x < width {
                pixels[cur.y * width + cur.x] = b;
            }
            cur.x += 1;
            cur.pos += 1;
            continue;
        }
        if !handle_rle_escape(data, &mut cur, &mut pixels, width) {
            break;
        }
    }
    pixels
}

struct RleCursor {
    x: usize,
    y: usize,
    pos: usize,
}

/// Handle one RLE escape sequence (the byte after a 0x00 marker).
/// Returns false when the data is exhausted.
fn handle_rle_escape(data: &[u8], cur: &mut RleCursor, pixels: &mut [u8], width: usize) -> bool {
    cur.pos += 1;
    if cur.pos >= data.len() {
        return false;
    }
    let flag = data[cur.pos];
    if flag == 0 {
        cur.x = 0;
        cur.y += 1;
        cur.pos += 1;
    } else if flag < 0x40 {
        cur.x += flag as usize;
        cur.pos += 1;
    } else if flag < 0x80 {
        cur.x += (((flag & 0x3F) as usize) << 8) | data[cur.pos + 1] as usize;
        cur.pos += 2;
    } else if flag < 0xC0 {
        let n = (flag & 0x3F) as usize;
        fill_pixels(pixels, width, cur.y, cur.x, n, data[cur.pos + 1]);
        cur.x += n;
        cur.pos += 2;
    } else {
        let n = (((flag & 0x3F) as usize) << 8) | data[cur.pos + 1] as usize;
        fill_pixels(pixels, width, cur.y, cur.x, n, data[cur.pos + 2]);
        cur.x += n;
        cur.pos += 3;
    }
    true
}

fn fill_pixels(pixels: &mut [u8], width: usize, y: usize, x: usize, n: usize, color: u8) {
    for i in 0..n {
        let px = x + i;
        if px < width && y * width + px < pixels.len() {
            pixels[y * width + px] = color;
        }
    }
}

/// BT.601 YUV to RGB (PGS uses BT.601 for HD, BT.709 for UHD).
fn yuv_to_rgb(y: u8, cr: u8, cb: u8) -> (u8, u8, u8) {
    let y = y as f64;
    let cr = cr as f64 - 128.0;
    let cb = cb as f64 - 128.0;
    let r = y + 1.402 * cr;
    let g = y - 0.344136 * cb - 0.714136 * cr;
    let b = y + 1.772 * cb;
    (clamp(r), clamp(g), clamp(b))
}

fn clamp(v: f64) -> u8 {
    v.round().max(0.0).min(255.0) as u8
}
