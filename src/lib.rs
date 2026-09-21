use heif::{Chroma, HeifEncoder, Preset};
use image::codecs::jpeg::JpegEncoder;
use image::{ExtendedColorType, ImageEncoder};
use std::os::raw::c_uchar;

// ===========================================================================
// HEIC → JPEG（解码）
// ===========================================================================

/// HEIC → JPEG 转码。
///
/// 成功返回 0；失败返回负值：
///   -1: 参数非法
///   -2: HEIC 读取/解码失败
///   -3: JPEG 编码失败
///
/// 输出字节由 Rust 分配，调用方必须调用 [heic_free] 释放。
#[no_mangle]
pub unsafe extern "C" fn heic_to_jpeg(
    input: *const c_uchar,
    input_len: usize,
    quality: u8,
    output: *mut *mut c_uchar,
    output_len: *mut usize,
) -> i32 {
    if input.is_null() || output.is_null() || output_len.is_null() || input_len == 0 {
        return -1;
    }
    *output = std::ptr::null_mut();
    *output_len = 0;

    let input_slice = std::slice::from_raw_parts(input, input_len);

    // 1. 用 heif-rs 解码 HEIC → DynamicImage
    let img = match heif::decode(input_slice) {
        Ok(i) => i,
        Err(_) => return -2,
    };

    // 2. 编码为 JPEG
    let mut jpeg_buf = Vec::new();
    let mut encoder = JpegEncoder::new_with_quality(&mut jpeg_buf, quality);
    if encoder
        .encode(
            img.as_bytes(),
            img.width(),
            img.height(),
            ExtendedColorType::Rgb8,
        )
        .is_err()
    {
        return -3;
    }

    // 3. 所有权转移到 C 侧
    let mut boxed = jpeg_buf.into_boxed_slice();
    *output = boxed.as_mut_ptr();
    *output_len = boxed.len();
    std::mem::forget(boxed);

    0
}

// ===========================================================================
// JPEG → HEIC（编码）—— 保持不变
// ===========================================================================

/// JPEG → HEIC 转码（HEVC 编码）。
///
/// 成功返回 0；失败返回负值：
///   -1: 参数非法
///   -2: JPEG 解码失败
///   -3: HEIC 编码失败
///
/// 输出字节由 Rust 分配，调用方必须调用 [heic_free] 释放。
#[no_mangle]
pub unsafe extern "C" fn jpeg_to_heic(
    input: *const c_uchar,
    input_len: usize,
    quality: u8,
    output: *mut *mut c_uchar,
    output_len: *mut usize,
) -> i32 {
    if input.is_null() || output.is_null() || output_len.is_null() || input_len == 0 {
        return -1;
    }
    *output = std::ptr::null_mut();
    *output_len = 0;

    let input_slice = std::slice::from_raw_parts(input, input_len);

    // 1. 解码 JPEG
    let img = match image::load_from_memory_with_format(
        input_slice,
        image::ImageFormat::Jpeg,
    ) {
        Ok(i) => i,
        Err(_) => return -2,
    };

    // 2. HEVC 编码为 HEIC
    let mut heic_buf = Vec::new();
    let encoder = HeifEncoder::new(&mut heic_buf)
        .with_quality(quality)          // 0–100
        .with_preset(Preset::Fast)      // x265 预设
        .with_chroma(Chroma::Yuv420);   // 4:2:0，iOS 兼容性最好

    if img.write_with_encoder(encoder).is_err() {
        return -3;
    }

    // 3. 所有权转移到 C 侧
    let mut boxed = heic_buf.into_boxed_slice();
    *output = boxed.as_mut_ptr();
    *output_len = boxed.len();
    std::mem::forget(boxed);

    0
}

// ===========================================================================
// 内存释放
// ===========================================================================

/// 释放 [heic_to_jpeg] 或 [jpeg_to_heic] 分配的输出缓冲。
///
/// 必须由调用方在拷贝完数据后调用一次。
#[no_mangle]
pub unsafe extern "C" fn heic_free(ptr: *mut c_uchar, len: usize) {
    if !ptr.is_null() && len > 0 {
        let _ = Vec::from_raw_parts(ptr, len, len);
    }
}