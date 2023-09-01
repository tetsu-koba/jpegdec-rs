pub fn decode_jpeg_to_i422(
    jpeg_data: &[u8],
    i422_data: &mut [u8],
    width: u32,
    height: u32,
) -> Result<(), Box<dyn std::error::Error>> {
    // initialize a decompressor
    let mut decompressor = turbojpeg::Decompressor::new()?;

    // read the JPEG header
    let header = decompressor.read_header(jpeg_data)?;
    if header.width != width as _ || header.height != height as _ {
        return Err("Framesize mismatch.".into());
    }
    // calculate YUV pixels length
    let align = 1;
    let yuv_pixels_len =
        turbojpeg::yuv_pixels_len(header.width, align, header.height, header.subsamp).unwrap();
    if yuv_pixels_len > i422_data.len() {
        return Err("Not enough bufsize.".into());
    }

    // initialize the image (YuvImage<Vec<u8>>)
    let mut image = turbojpeg::YuvImage {
        pixels: i422_data,
        width: header.width,
        align,
        height: header.height,
        subsamp: header.subsamp,
    };

    // decompress the JPEG into the image
    decompressor.decompress_to_yuv(&jpeg_data, image.as_deref_mut())?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_jpeg_to_i422() -> Result<(), Box<dyn std::error::Error>> {
        let input_jpeg_path = "testfiles/test001.jpeg";
        let expected_output_path = "testfiles/test001.i422";

        // read JPEG data from file
        let jpeg_data = std::fs::read(input_jpeg_path)?;
        let width: u32 = 160;
        let height: u32 = 90;
        let mut i422_data = vec![0u8; (width * height * 2) as _];
        decode_jpeg_to_i422(&jpeg_data, &mut i422_data, width, height)?;

        let expected_data = std::fs::read(expected_output_path)?;
        assert_eq!(i422_data, expected_data);

        Ok(())
    }
}
