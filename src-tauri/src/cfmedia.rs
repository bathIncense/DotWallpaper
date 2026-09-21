// 直接声明 CoreFoundation / ImageIO 的 C 接口，用于缩略图与 JPEG 编码。
// 不引入 Rust 编解码库；系统框架不随应用打包。全部为纯 C 函数，线程安全。

#![allow(non_upper_case_globals, non_snake_case, dead_code)]

use std::ffi::c_void;

pub type CFTypeRef = *const c_void;

const kCFStringEncodingUTF8: u32 = 0x0800_0100;
const kCFNumberFloat64Type: u32 = 6;
const kCFURLPOSIXPathStyle: u32 = 0;

#[repr(C)]
pub struct OpaqueCallbacks([u8; 0]);

extern "C" {
    fn CFStringCreateWithCString(alloc: CFTypeRef, cStr: *const u8, encoding: u32) -> *mut c_void;
    fn CFURLCreateWithFileSystemPath(
        alloc: CFTypeRef,
        filePath: CFTypeRef,
        pathStyle: u32,
        isDirectory: bool,
    ) -> *mut c_void;
    fn CFDictionaryCreate(
        alloc: CFTypeRef,
        keys: *const CFTypeRef,
        values: *const CFTypeRef,
        numValues: i64,
        callBacks: *const c_void,
        valueCallBacks: *const c_void,
    ) -> *mut c_void;
    fn CFNumberCreate(alloc: CFTypeRef, theType: u32, valuePtr: *const c_void) -> *mut c_void;
    fn CFRelease(cf: *mut c_void);
    static kCFTypeDictionaryKeyCallBacks: OpaqueCallbacks;
    static kCFTypeDictionaryValueCallBacks: OpaqueCallbacks;
    static kCFBooleanTrue: CFTypeRef;

    fn CGImageSourceCreateWithURL(imageURL: CFTypeRef, options: CFTypeRef) -> *mut c_void;
    fn CGImageSourceCreateImageAtIndex(
        src: CFTypeRef,
        index: usize,
        options: CFTypeRef,
    ) -> *mut c_void;
    fn CGImageSourceCreateThumbnailAtIndex(
        src: CFTypeRef,
        index: usize,
        options: CFTypeRef,
    ) -> *mut c_void;
    fn CGImageGetWidth(image: CFTypeRef) -> usize;
    fn CGImageGetHeight(image: CFTypeRef) -> usize;
    fn CGImageDestinationCreateWithURL(
        url: CFTypeRef,
        type_: CFTypeRef,
        count: usize,
        options: CFTypeRef,
    ) -> *mut c_void;
    fn CGImageDestinationAddImage(dst: CFTypeRef, image: CFTypeRef, properties: CFTypeRef);
    fn CGImageDestinationFinalize(dst: CFTypeRef) -> u8;
}

fn c_string(s: &str) -> *mut c_void {
    let mut bytes = s.as_bytes().to_vec();
    bytes.push(0);
    unsafe { CFStringCreateWithCString(std::ptr::null(), bytes.as_ptr(), kCFStringEncodingUTF8) }
}

fn url_for(path: &str) -> *mut c_void {
    let s = c_string(path);
    let url = unsafe {
        CFURLCreateWithFileSystemPath(std::ptr::null(), s as *const c_void, kCFURLPOSIXPathStyle, false)
    };
    unsafe { CFRelease(s) };
    url
}

fn dictionary(pairs: &[(&str, *const c_void)]) -> *mut c_void {
    let keys: Vec<*mut c_void> = pairs.iter().map(|(k, _)| c_string(k)).collect();
    let values: Vec<*const c_void> = pairs.iter().map(|(_, v)| *v).collect();
    let dict = unsafe {
        CFDictionaryCreate(
            std::ptr::null(),
            keys.as_ptr() as *const *const c_void,
            values.as_ptr(),
            pairs.len() as i64,
            &kCFTypeDictionaryKeyCallBacks as *const _ as *const c_void,
            &kCFTypeDictionaryValueCallBacks as *const _ as *const c_void,
        )
    };
    for k in keys {
        unsafe { CFRelease(k) };
    }
    dict
}

fn number_f64(v: f64) -> *mut c_void {
    unsafe { CFNumberCreate(std::ptr::null(), kCFNumberFloat64Type, &v as *const _ as *const c_void) }
}

/// 读取图片像素尺寸（不解码全图）。
pub fn image_size(path: &str) -> Option<(usize, usize)> {
    unsafe {
        let url = url_for(path);
        let src = CGImageSourceCreateWithURL(url as *const c_void, std::ptr::null());
        CFRelease(url);
        if src.is_null() {
            return None;
        }
        let img = CGImageSourceCreateImageAtIndex(src as *const c_void, 0, std::ptr::null());
        let size = if img.is_null() {
            None
        } else {
            Some((CGImageGetWidth(img as *const c_void), CGImageGetHeight(img as *const c_void)))
        };
        if !img.is_null() {
            CFRelease(img);
        }
        CFRelease(src);
        size
    }
}

/// 将 CGImage（不透明指针）编码为 JPEG 文件。
pub fn encode_jpeg(cg_image: *const c_void, dst_path: &str, quality: f64) -> Result<(), String> {
    if cg_image.is_null() {
        return Err("CGImage 为空".to_string());
    }
    unsafe {
        let url = url_for(dst_path);
        let kind = c_string("public.jpeg");
        let dst = CGImageDestinationCreateWithURL(url as *const c_void, kind as *const c_void, 1, std::ptr::null());
        CFRelease(url);
        CFRelease(kind);
        if dst.is_null() {
            return Err("无法创建 JPEG 编码目标".to_string());
        }
        let q = number_f64(quality);
        let props = dictionary(&[("kCGImageDestinationLossyCompressionQuality", q as *const c_void)]);
        CFRelease(q);
        CGImageDestinationAddImage(dst as *const c_void, cg_image, props as *const c_void);
        if !props.is_null() {
            CFRelease(props);
        }
        let ok = CGImageDestinationFinalize(dst as *const c_void);
        CFRelease(dst);
        if ok == 0 {
            return Err("JPEG 编码失败".to_string());
        }
    }
    Ok(())
}

/// 从源图片文件生成等比缩略图 JPEG（最长边 max_px，自动应用 EXIF 方向）。
pub fn make_thumbnail_jpeg(src_path: &str, dst_path: &str, max_px: u32) -> Result<(), String> {
    unsafe {
        let url = url_for(src_path);
        let src = CGImageSourceCreateWithURL(url as *const c_void, std::ptr::null());
        CFRelease(url);
        if src.is_null() {
            return Err("ImageIO 无法读取图片".to_string());
        }
        let k_always = c_string("kCGImageSourceCreateThumbnailFromImageAlways");
        let k_transform = c_string("kCGImageSourceCreateThumbnailWithTransform");
        let k_max = c_string("kCGImageSourceThumbnailMaxPixelSize");
        let max_num = number_f64(max_px as f64);
        let opts = dictionary(&[
            ("kCGImageSourceCreateThumbnailFromImageAlways", kCFBooleanTrue),
            ("kCGImageSourceCreateThumbnailWithTransform", kCFBooleanTrue),
            ("kCGImageSourceThumbnailMaxPixelSize", max_num as *const c_void),
        ]);
        let thumb =
            CGImageSourceCreateThumbnailAtIndex(src as *const c_void, 0, opts as *const c_void);
        CFRelease(opts);
        CFRelease(max_num);
        CFRelease(k_always);
        CFRelease(k_transform);
        CFRelease(k_max);
        if thumb.is_null() {
            CFRelease(src);
            return Err("缩略图生成失败".to_string());
        }
        let result = encode_jpeg(thumb as *const c_void, dst_path, 0.85);
        CFRelease(thumb);
        CFRelease(src);
        result
    }
}
