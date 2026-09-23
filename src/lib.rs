use image::codecs::jpeg::JpegEncoder;
use image::ExtendedColorType;
use libheif_rs::{
    Channel, ColorSpace, CompressionFormat, EncoderQuality, EncodingOptions,
    HeifContext, LibHeif, RgbChroma,
};
use std::os::raw::c_uchar;

// ===========================================================================
// HEIC → JPEG（解码）
// ===========================================================================

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

    let lib_heif = LibHeif::new();
    let ctx = match HeifContext::read_from_bytes(input_slice) {
        Ok(c) => c,
        Err(_) => return -2,
    };
    let handle = match ctx.primary_image_handle() {
        Ok(h) => h,
        Err(_) => return -3,
    };
    let image = match lib_heif.decode(&handle, ColorSpace::Rgb(RgbChroma::Rgb), None) {
        Ok(i) => i,
        Err(_) => return -4,
    };

    let planes = image.planes();
    let plane = match planes.interleaved {
        Some(p) => p,
        None => return -5,
    };

    let width = plane.width;
    let height = plane.height;
    let stride = plane.stride;
    let row_bytes = (width * 3) as usize;

    if plane.data.len() < (height as usize).saturating_mul(stride) {
        return -5;
    }

    let mut rgb = Vec::with_capacity(row_bytes * height as usize);
    for y in 0..height {
        let start = (y as usize) * stride;
        let end = start + row_bytes;
        rgb.extend_from_slice(&plane.data[start..end]);
    }

    let mut jpeg_buf = Vec::new();
    let mut encoder = JpegEncoder::new_with_quality(&mut jpeg_buf, quality);
    if encoder
        .encode(&rgb, width, height, ExtendedColorType::Rgb8)
        .is_err()
    {
        return -6;
    }

    let mut boxed = jpeg_buf.into_boxed_slice();
    *output = boxed.as_mut_ptr();
    *output_len = boxed.len();
    std::mem::forget(boxed);

    0
}

// ===========================================================================
// JPEG → HEIC（编码 + EXIF 注入）
// ===========================================================================

/// JPEG → HEIC，同时注入 EXIF（含 Apple MakerNote + UUID）。
///
/// [exif] 是 ExifDataBlock 格式：前 4 字节是 TIFF header 偏移，后跟 TIFF。
///
/// 成功返回 0；失败返回负值：
///   -1: 参数非法
///   -2: JPEG 解码失败
///   -3: 创建 HEIC 上下文/图像失败
///   -4: 获取或配置 HEVC 编码器失败
///   -5: 编码失败
///   -6: 注入 EXIF 失败
///   -7: 写出字节失败
#[no_mangle]
pub unsafe extern "C" fn jpeg_to_heic_with_uuid(
    input: *const c_uchar,
    input_len: usize,
    exif: *const c_uchar,
    exif_len: usize,
    quality: u8,
    output: *mut *mut c_uchar,
    output_len: *mut usize,
) -> i32 {
    if input.is_null()
        || output.is_null()
        || output_len.is_null()
        || input_len == 0
        || exif.is_null()
        || exif_len == 0
    {
        return -1;
    }
    *output = std::ptr::null_mut();
    *output_len = 0;

    let input_slice = std::slice::from_raw_parts(input, input_len);
    let exif_slice = std::slice::from_raw_parts(exif, exif_len);

    // 1. 解码 JPEG
    let img = match image::load_from_memory_with_format(
        input_slice,
        image::ImageFormat::Jpeg,
    ) {
        Ok(i) => i,
        Err(_) => return -2,
    };
    let rgb = img.to_rgb8();
    let width = rgb.width();
    let height = rgb.height();

    // 2. 创建 libheif 图像并填充像素
    let lib_heif = LibHeif::new();
    let mut heif_image = match libheif_rs::Image::new(
        width,
        height,
        ColorSpace::Rgb(RgbChroma::Rgb),
    ) {
        Ok(i) => i,
        Err(_) => return -3,
    };
    if heif_image
        .create_plane(Channel::Interleaved, width, height, 8)
        .is_err()
    {
        return -3;
    }
    {
        let mut planes = heif_image.planes_mut();
        let plane = match planes.interleaved {
            Some(p) => p,
            None => return -3,
        };
        let src = rgb.as_raw();
        let width_usize = width as usize;
        let height_usize = height as usize;
        for y in 0..height_usize {
            let src_start = y * width_usize * 3;
            let dst_start = y * plane.stride;
            let len = width_usize * 3;
            plane.data[dst_start..dst_start + len]
                .copy_from_slice(&src[src_start..src_start + len]);
        }
    }

    // 3. 创建上下文 + 编码器
    let mut ctx = match HeifContext::new() {
        Ok(c) => c,
        Err(_) => return -3,
    };
    let mut encoder = match lib_heif.encoder_for_format(CompressionFormat::Hevc) {
        Ok(e) => e,
        Err(_) => return -4,
    };
    if encoder
        .set_quality(EncoderQuality::Lossy(quality))
        .is_err()
    {
        return -4;
    }

    // 4. 编码
    let encoding_options = match EncodingOptions::new() {
        Ok(o) => o,
        Err(_) => return -5,
    };
    let handle = match ctx.encode_image(&heif_image, &mut encoder, Some(encoding_options)) {
        Ok(h) => h,
        Err(_) => return -5,
    };

    // 5. 注入 EXIF
    //    libheif 要求 EXIF 数据前 4 字节是 TIFF header 偏移，
    //    这正是 ExifDataBlock 的格式，直接传入即可。
    if ctx.add_exif_metadata(&handle, exif_slice).is_err() {
        return -6;
    }

    // 6. 写出
    let heic_buf = match ctx.write_to_bytes() {
        Ok(b) => b,
        Err(_) => return -7,
    };

    let mut boxed = heic_buf.into_boxed_slice();
    *output = boxed.as_mut_ptr();
    *output_len = boxed.len();
    std::mem::forget(boxed);

    0
}

// ===========================================================================
// 内存释放
// ===========================================================================

/// 释放 [heic_to_jpeg] 或 [jpeg_to_heic_with_uuid] 分配的输出缓冲。
#[no_mangle]
pub unsafe extern "C" fn heic_free(ptr: *mut c_uchar, len: usize) {
    if !ptr.is_null() && len > 0 {
        let _ = Vec::from_raw_parts(ptr, len, len);
    }
}
