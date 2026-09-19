use image::codecs::jpeg::JpegEncoder;
use image::ExtendedColorType;
use libheif_rs::{ColorSpace, HeifContext, RgbChroma};
use std::os::raw::c_uchar;

/// HEIC → JPEG 转码。
///
/// 成功返回 0；失败返回负值：
///   -1: 参数非法
///   -2: HEIC 读取失败
///   -3: 获取主图失败
///   -4: HEVC 解码失败
///   -5: 像素数据异常
///   -6: JPEG 编码失败
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

    // 1. 读 HEIC 容器
    let ctx = match HeifContext::read_from_bytes(input_slice) {
        Ok(c) => c,
        Err(_) => return -2,
    };

    // 2. 取主图
    let handle = match ctx.primary_image_handle() {
        Ok(h) => h,
        Err(_) => return -3,
    };

    // 3. HEVC 解码 → RGB
    let image = match handle.decode(ColorSpace::Rgb(RgbChroma::Rgb), false) {
        Ok(img) => img,
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

    if plane.data.len() < (height as usize).saturating_mul(stride as usize) {
        return -5;
    }

    // 逐行拷贝，去掉 stride padding
    let mut rgb = Vec::with_capacity(row_bytes * height as usize);
    for y in 0..height {
        let start = (y * stride) as usize;
        let end = start + row_bytes;
        rgb.extend_from_slice(&plane.data[start..end]);
    }

    // 4. JPEG 编码
    let mut jpeg_buf = Vec::new();
    let mut encoder = JpegEncoder::new_with_quality(&mut jpeg_buf, quality);
    if encoder
        .encode(&rgb, width, height, ExtendedColorType::Rgb8)
        .is_err()
    {
        return -6;
    }

    // 5. 所有权转移到 C 侧
    let mut boxed = jpeg_buf.into_boxed_slice();
    *output = boxed.as_mut_ptr();use image::codecs::jpeg::JpegEncoder;
    use image::ExtendedColorType;
    use libheif_rs::{ColorSpace, HeifContext, RgbChroma};
    use std::os::raw::c_uchar;

    /// HEIC → JPEG 转码。
    ///
    /// 成功返回 0；失败返回负值：
    ///   -1: 参数非法
    ///   -2: HEIC 读取失败
    ///   -3: 获取主图失败
    ///   -4: HEVC 解码失败
    ///   -5: 像素数据异常
    ///   -6: JPEG 编码失败
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

        // 1. 读 HEIC 容器
        let ctx = match HeifContext::read_from_bytes(input_slice) {
            Ok(c) => c,
            Err(_) => return -2,
        };

        // 2. 取主图
        let handle = match ctx.primary_image_handle() {
            Ok(h) => h,
            Err(_) => return -3,
        };

        // 3. HEVC 解码 → RGB
        let image = match handle.decode(ColorSpace::Rgb(RgbChroma::Rgb), false) {
            Ok(img) => img,
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

        if plane.data.len() < (height as usize).saturating_mul(stride as usize) {
            return -5;
        }

        // 逐行拷贝，去掉 stride padding
        let mut rgb = Vec::with_capacity(row_bytes * height as usize);
        for y in 0..height {
            let start = (y * stride) as usize;
            let end = start + row_bytes;
            rgb.extend_from_slice(&plane.data[start..end]);
        }

        // 4. JPEG 编码
        let mut jpeg_buf = Vec::new();
        let mut encoder = JpegEncoder::new_with_quality(&mut jpeg_buf, quality);
        if encoder
            .encode(&rgb, width, height, ExtendedColorType::Rgb8)
            .is_err()
        {
            return -6;
        }

        // 5. 所有权转移到 C 侧
        let mut boxed = jpeg_buf.into_boxed_slice();
        *output = boxed.as_mut_ptr();
        *output_len = boxed.len();
        std::mem::forget(boxed);

        0
    }

    /// 释放 [heic_to_jpeg] 分配的输出缓冲。
    ///
    /// 必须由调用方在拷贝完数据后调用一次。
    #[no_mangle]
    pub unsafe extern "C" fn heic_free(ptr: *mut c_uchar, len: usize) {
        if !ptr.is_null() && len > 0 {
            let _ = Vec::from_raw_parts(ptr, len, len);
        }
    }
    *output_len = boxed.len();
    std::mem::forget(boxed);

    0
}

/// 释放 [heic_to_jpeg] 分配的输出缓冲。
///
/// 必须由调用方在拷贝完数据后调用一次。
#[no_mangle]
pub unsafe extern "C" fn heic_free(ptr: *mut c_uchar, len: usize) {
    if !ptr.is_null() && len > 0 {
        let _ = Vec::from_raw_parts(ptr, len, len);
    }
}