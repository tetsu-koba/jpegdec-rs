use std::env;
use std::fs::File;
use std::io::{self, Read, Write};
mod jpegdec;

#[cfg(target_os = "linux")]
mod pipe;

enum State {
    St0, // waiting for JPEG_START0
    St1, // waiting for JPEG_START1
    St2, // waiting for JPEG_END0
    St3, // waiting for JPEG_END1
}

const JPEG_START0: u8 = 0xff;
const JPEG_START1: u8 = 0xd8;
const JPEG_END0: u8 = 0xff;
const JPEG_END1: u8 = 0xd9;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();

    if args.len() < 5 {
        println!(
            "Usage: {} input_jpeg_file output_i422_file width height",
            args[0]
        );
        std::process::exit(1);
    }

    let mut infile = File::open(&args[1])?;
    let mut outfile = File::create(&args[2])?;
    let width: u32 = args[3].parse()?;
    let height: u32 = args[4].parse()?;

    decode_mjpeg(&mut infile, &mut outfile, width, height)?;

    Ok(())
}

fn decode_mjpeg(infile: &mut File, outfile: &mut File, width: u32, height: u32) -> io::Result<()> {
    #[cfg(target_os = "linux")]
    let bufsize = if pipe::is_pipe(infile) {
        pipe::get_pipe_max_size()?
    } else {
        64 * 1024
    };
    #[cfg(target_os = "linux")]
    let is_pipe_output = if pipe::is_pipe(outfile) {
        pipe::set_pipe_max_size(outfile)?;
        true
    } else {
        false
    };

    #[cfg(not(target_os = "linux"))]
    let bufsize = 64 * 1024;

    let mut buffer = vec![0u8; bufsize];
    let mut write_buffer = Vec::new();
    let mut i422_data = vec![0u8; (width * height * 2) as usize];
    let mut state = State::St0;

    'outer: loop {
        let n = infile.read(&mut buffer)?;
        if n == 0 {
            break;
        }

        for &v in &buffer[0..n] {
            match state {
                State::St0 => {
                    if v == JPEG_START0 {
                        state = State::St1;
                    }
                }
                State::St1 => {
                    if v == JPEG_START1 {
                        write_buffer.push(JPEG_START0);
                        write_buffer.push(JPEG_START1);
                        state = State::St2;
                    } else if v != JPEG_START0 {
                        state = State::St0;
                    }
                }
                State::St2 => {
                    write_buffer.push(v);
                    if v == JPEG_END0 {
                        state = State::St3;
                    }
                }
                State::St3 => {
                    write_buffer.push(v);
                    if v == JPEG_END1 {
                        state = State::St0;
                        if let Err(e) =
                            jpegdec::decode_to_i422(&write_buffer, &mut i422_data, width, height)
                        {
                            eprintln!("{e:?}");
                        } else {
                            #[cfg(target_os = "linux")]
                            let res = if is_pipe_output {
                                pipe::vmsplice_single_buffer(&i422_data, outfile)
                            } else {
                                outfile.write_all(&i422_data)
                            };

                            #[cfg(not(target_os = "linux"))]
                            let res = outfile.write_all(&i422_data);

                            if let Err(e) = res {
                                if e.kind() == std::io::ErrorKind::BrokenPipe {
                                    break 'outer;
                                }
                                return Err(e);
                            }
                        }
                        write_buffer.clear();
                    } else if v != JPEG_END0 {
                        state = State::St2;
                    }
                }
            }
        }
    }

    Ok(())
}
