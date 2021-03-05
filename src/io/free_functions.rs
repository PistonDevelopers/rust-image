use std::fs::File;
use std::io::{BufRead, BufReader, BufWriter, Seek};
use std::path::Path;
use std::u32;

#[cfg(feature = "bmp")]
use crate::codecs::bmp;
#[cfg(feature = "gif")]
use crate::codecs::gif;
#[cfg(feature = "hdr")]
use crate::codecs::hdr;
#[cfg(feature = "ico")]
use crate::codecs::ico;
#[cfg(feature = "jpeg")]
use crate::codecs::jpeg;
#[cfg(feature = "png")]
use crate::codecs::png;
#[cfg(feature = "pnm")]
use crate::codecs::pnm;
#[cfg(feature = "tga")]
use crate::codecs::tga;
#[cfg(feature = "dds")]
use crate::codecs::dds;
#[cfg(feature = "tiff")]
use crate::codecs::tiff;
#[cfg(feature = "webp")]
use crate::codecs::webp;
#[cfg(feature = "farbfeld")]
use crate::codecs::farbfeld;
#[cfg(any(feature = "avif-encoder", feature = "avif-decoder"))]
use crate::codecs::avif;

use crate::color;
use crate::image;
use crate::dynimage::DynamicImage;
use crate::error::{ImageError, ImageFormatHint, ImageResult};
use crate::image::ImageFormat;
#[allow(unused_imports)]  // When no features are supported
use crate::image::{ImageDecoder, ImageEncoder};

pub(crate) fn open_impl(path: &Path) -> ImageResult<DynamicImage> {
    let fin = match File::open(path) {
        Ok(f) => f,
        Err(err) => return Err(ImageError::IoError(err)),
    };
    let fin = BufReader::new(fin);

    load(fin, ImageFormat::from_path(path)?)
}

/// Create a new image from a Reader
///
/// Try [`io::Reader`] for more advanced uses.
///
/// [`io::Reader`]: io/struct.Reader.html
#[allow(unused_variables)]
// r is unused if no features are supported.
pub fn load<R: BufRead + Seek>(r: R, format: ImageFormat) -> ImageResult<DynamicImage> {
    load_inner(r, super::Limits::default(), format)
}

pub(crate) trait DecoderVisitor {
    type Result;
    fn visit_decoder<'a, D: ImageDecoder<'a>>(self, decoder: D) -> ImageResult<Self::Result>;
}

pub(crate) fn load_decoder<R: BufRead + Seek, V: DecoderVisitor>(r: R, format: ImageFormat, visitor: V) -> ImageResult<V::Result> {
    #[allow(unreachable_patterns)]
    // Default is unreachable if all features are supported.
    match format {
        #[cfg(feature = "avif-decoder")]
        image::ImageFormat::Avif => visitor.visit_decoder(avif::AvifDecoder::new(r)?),
        #[cfg(feature = "png")]
        image::ImageFormat::Png => visitor.visit_decoder(png::PngDecoder::new(r)?),
        #[cfg(feature = "gif")]
        image::ImageFormat::Gif => visitor.visit_decoder(gif::GifDecoder::new(r)?),
        #[cfg(feature = "jpeg")]
        image::ImageFormat::Jpeg => visitor.visit_decoder(jpeg::JpegDecoder::new(r)?),
        #[cfg(feature = "webp")]
        image::ImageFormat::WebP => visitor.visit_decoder(webp::WebPDecoder::new(r)?),
        #[cfg(feature = "tiff")]
        image::ImageFormat::Tiff => visitor.visit_decoder(tiff::TiffDecoder::new(r)?),
        #[cfg(feature = "tga")]
        image::ImageFormat::Tga => visitor.visit_decoder(tga::TgaDecoder::new(r)?),
        #[cfg(feature = "dds")]
        image::ImageFormat::Dds => visitor.visit_decoder(dds::DdsDecoder::new(r)?),
        #[cfg(feature = "bmp")]
        image::ImageFormat::Bmp => visitor.visit_decoder(bmp::BmpDecoder::new(r)?),
        #[cfg(feature = "ico")]
        image::ImageFormat::Ico => visitor.visit_decoder(ico::IcoDecoder::new(r)?),
        #[cfg(feature = "hdr")]
        image::ImageFormat::Hdr => visitor.visit_decoder(hdr::HdrAdapter::new(BufReader::new(r))?),
        #[cfg(feature = "pnm")]
        image::ImageFormat::Pnm => visitor.visit_decoder(pnm::PnmDecoder::new(r)?),
        #[cfg(feature = "farbfeld")]
        image::ImageFormat::Farbfeld => visitor.visit_decoder(farbfeld::FarbfeldDecoder::new(r)?),
        _ => Err(ImageError::Unsupported(ImageFormatHint::Exact(format).into())),
    }
}

pub(crate) fn load_inner<R: BufRead + Seek>(r: R, limits: super::Limits, format: ImageFormat) -> ImageResult<DynamicImage> {
    struct LoadVisitor(super::Limits);

    impl DecoderVisitor for LoadVisitor {
        type Result = DynamicImage;

        fn visit_decoder<'a, D: ImageDecoder<'a>>(self, mut decoder: D) -> ImageResult<Self::Result> {
            let mut limits = self.0;
            let total_bytes = decoder.total_bytes();
            // Check that we do not allocate a bigger buffer than we are allowed to
            // FIXME: should this rather go in `DynamicImage::from_decoder` somehow?
            if let Some(max_alloc) = limits.max_alloc.as_mut() {
                if *max_alloc < total_bytes {
                    return Err(ImageError::Limits(crate::error::LimitError::from_kind(
                        crate::error::LimitErrorKind::InsufficientMemory)))
                }
                // We are allocating a buffer of size `total_bytes` outside of
                // the decoder. Therefore the decoder gets a smaller limit.
                *max_alloc -= total_bytes;
            }
            decoder.set_limits(self.0)?;
            DynamicImage::from_decoder(decoder)
        }
    }

    load_decoder(r, format, LoadVisitor(limits))
}

pub(crate) fn image_dimensions_impl(path: &Path) -> ImageResult<(u32, u32)> {
    let format = image::ImageFormat::from_path(path)?;

    let fin = File::open(path)?;
    let fin = BufReader::new(fin);

    image_dimensions_with_format_impl(fin, format)
}

#[allow(unused_variables)]
// fin is unused if no features are supported.
pub(crate) fn image_dimensions_with_format_impl<R: BufRead + Seek>(fin: R, format: ImageFormat)
    -> ImageResult<(u32, u32)>
{
        struct DimVisitor;

        impl DecoderVisitor for DimVisitor {
            type Result = (u32, u32);
            fn visit_decoder<'a, D: ImageDecoder<'a>>(self, decoder: D) -> ImageResult<Self::Result> {
                Ok(decoder.dimensions())
            }
        }

        load_decoder(fin, format, DimVisitor)
}

#[allow(unused_variables)]
// Most variables when no features are supported
pub(crate) fn save_buffer_impl(
    path: &Path,
    buf: &[u8],
    width: u32,
    height: u32,
    color: color::ColorType,
) -> ImageResult<()> {
    let fout = &mut BufWriter::new(File::create(path)?);
    let format =  ImageFormat::from_path(path)?;
    save_buffer_with_format_impl(path, buf, width, height, color, format)
}

#[allow(unused_variables)]
// Most variables when no features are supported
pub(crate) fn save_buffer_with_format_impl(
    path: &Path,
    buf: &[u8],
    width: u32,
    height: u32,
    color: color::ColorType,
    format: ImageFormat,
) -> ImageResult<()> {
    let fout = &mut BufWriter::new(File::create(path)?);

    match format {
        #[cfg(feature = "gif")]
        image::ImageFormat::Gif => gif::GifEncoder::new(fout).encode(buf, width, height, color),
        #[cfg(feature = "ico")]
        image::ImageFormat::Ico => ico::IcoEncoder::new(fout).write_image(buf, width, height, color),
        #[cfg(feature = "jpeg")]
        image::ImageFormat::Jpeg => jpeg::JpegEncoder::new(fout).write_image(buf, width, height, color),
        #[cfg(feature = "png")]
        image::ImageFormat::Png => png::PngEncoder::new(fout).write_image(buf, width, height, color),
        #[cfg(feature = "pnm")]
        image::ImageFormat::Pnm => {
            let ext = path.extension()
            .and_then(|s| s.to_str())
            .map_or("".to_string(), |s| s.to_ascii_lowercase());
            match &*ext {
                "pbm" => pnm::PnmEncoder::new(fout)
                    .with_subtype(pnm::PNMSubtype::Bitmap(pnm::SampleEncoding::Binary))
                    .write_image(buf, width, height, color),
                "pgm" => pnm::PnmEncoder::new(fout)
                    .with_subtype(pnm::PNMSubtype::Graymap(pnm::SampleEncoding::Binary))
                    .write_image(buf, width, height, color),
                "ppm" => pnm::PnmEncoder::new(fout)
                    .with_subtype(pnm::PNMSubtype::Pixmap(pnm::SampleEncoding::Binary))
                    .write_image(buf, width, height, color),
                "pam" => pnm::PnmEncoder::new(fout).write_image(buf, width, height, color),
                _ => Err(ImageError::Unsupported(ImageFormatHint::Exact(format).into())), // Unsupported Pnm subtype.
            }
        },
        #[cfg(feature = "farbfeld")]
        image::ImageFormat::Farbfeld => farbfeld::FarbfeldEncoder::new(fout).write_image(buf, width, height, color),
        #[cfg(feature = "avif-encoder")]
        image::ImageFormat::Avif => avif::AvifEncoder::new(fout).write_image(buf, width, height, color),
        // #[cfg(feature = "hdr")]
        // image::ImageFormat::Hdr => hdr::HdrEncoder::new(fout).encode(&[Rgb<f32>], width, height), // usize
        #[cfg(feature = "bmp")]
        image::ImageFormat::Bmp => bmp::BmpEncoder::new(fout).write_image(buf, width, height, color),
        #[cfg(feature = "tiff")]
        image::ImageFormat::Tiff => tiff::TiffEncoder::new(fout)
            .write_image(buf, width, height, color),
        #[cfg(feature = "tga")]
        image::ImageFormat::Tga => tga::TgaEncoder::new(fout).write_image(buf, width, height, color),
        format => Err(ImageError::Unsupported(ImageFormatHint::Exact(format).into())),
    }
}

static MAGIC_BYTES: [(&[u8], ImageFormat); 20] = [
    (b"\x89PNG\r\n\x1a\n", ImageFormat::Png),
    (&[0xff, 0xd8, 0xff], ImageFormat::Jpeg),
    (b"GIF89a", ImageFormat::Gif),
    (b"GIF87a", ImageFormat::Gif),
    (b"RIFF", ImageFormat::WebP), // TODO: better magic byte detection, see https://github.com/image-rs/image/issues/660
    (b"MM\x00*", ImageFormat::Tiff),
    (b"II*\x00", ImageFormat::Tiff),
    (b"DDS ", ImageFormat::Dds),
    (b"BM", ImageFormat::Bmp),
    (&[0, 0, 1, 0], ImageFormat::Ico),
    (b"#?RADIANCE", ImageFormat::Hdr),
    (b"P1", ImageFormat::Pnm),
    (b"P2", ImageFormat::Pnm),
    (b"P3", ImageFormat::Pnm),
    (b"P4", ImageFormat::Pnm),
    (b"P5", ImageFormat::Pnm),
    (b"P6", ImageFormat::Pnm),
    (b"P7", ImageFormat::Pnm),
    (b"farbfeld", ImageFormat::Farbfeld),
    (b"\0\0\0 ftypavif", ImageFormat::Avif),
];

/// Guess image format from memory block
///
/// Makes an educated guess about the image format based on the Magic Bytes at the beginning.
/// TGA is not supported by this function.
/// This is not to be trusted on the validity of the whole memory block
pub fn guess_format(buffer: &[u8]) -> ImageResult<ImageFormat> {
    match guess_format_impl(buffer) {
        Some(format) => Ok(format),
        None => Err(ImageError::Unsupported(ImageFormatHint::Unknown.into())),
    }
}

pub(crate) fn guess_format_impl(buffer: &[u8]) -> Option<ImageFormat> {
    for &(signature, format) in &MAGIC_BYTES {
        if buffer.starts_with(signature) {
            return Some(format);
        }
    }

    None
}
