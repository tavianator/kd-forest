pub mod color;
pub mod forest;
pub mod frontier;
pub mod hilbert;
pub mod soft;

use crate::color::source::{AllColors, ColorSource, ImageColors};
use crate::color::{order, ColorSpace, LabSpace, LuvSpace, Rgb8, RgbSpace};
use crate::frontier::image::ImageFrontier;
use crate::frontier::mean::MeanFrontier;
use crate::frontier::min::MinFrontier;
use crate::frontier::Frontier;

use clap::{self, clap_app, crate_authors, crate_name, crate_version};

use image::{self, ColorType, ImageError, Rgba, RgbaImage};
use image::png::{CompressionType, FilterType, PngEncoder};

use rand::{self, SeedableRng};
use rand_pcg::Pcg64;

use std::cmp;
use std::error::Error;
use std::io::{self, BufWriter, IsTerminal, Write};
use std::path::PathBuf;
use std::process::exit;
use std::str::FromStr;
use std::time::Instant;

/// The color source specified on the command line.
#[derive(Debug, Eq, PartialEq)]
enum SourceArg {
    /// All RGB colors of the given bit depth(s).
    AllRgb(u32, u32, u32),
    /// Take the colors from an image.
    Image(PathBuf),
}

/// The order to process colors in.
#[derive(Debug, Eq, PartialEq)]
enum OrderArg {
    /// Sorted by hue.
    HueSort,
    /// Shuffled randomly.
    Random,
    /// Morton/Z-order.
    Morton,
    /// Hilbert curve order.
    Hilbert,
}

/// The frontier implementation.
#[derive(Debug, Eq, PartialEq)]
enum FrontierArg {
    /// Pick a neighbor of the closest pixel so far.
    Min,
    /// Pick the pixel with the closest mean color of all its neighbors.
    Mean,
    /// Target the closest pixel on an image.
    Image(PathBuf),
}

/// The color space to operate in.
#[derive(Debug, Eq, PartialEq)]
enum ColorSpaceArg {
    /// sRGB space.
    Rgb,
    /// CIE L*a*b* space.
    Lab,
    /// CIE L*u*v* space.
    Luv,
}

/// Error type for this app.
#[derive(Debug)]
enum AppError {
    ArgError(clap::Error),
    RuntimeError(Box<dyn Error>),
}

impl AppError {
    /// Create an error for an invalid argument.
    fn invalid_value(msg: &str) -> Self {
        Self::ArgError(clap::Error::with_description(
            msg,
            clap::ErrorKind::InvalidValue,
        ))
    }

    /// Exit the program with this error.
    fn exit(&self) -> ! {
        match self {
            Self::ArgError(err) => err.exit(),
            Self::RuntimeError(err) => {
                eprintln!("{}", err);
                exit(1)
            }
        }
    }
}

impl From<clap::Error> for AppError {
    fn from(err: clap::Error) -> Self {
        Self::ArgError(err)
    }
}

impl From<ImageError> for AppError {
    fn from(err: ImageError) -> Self {
        Self::RuntimeError(Box::new(err))
    }
}

impl From<io::Error> for AppError {
    fn from(err: io::Error) -> Self {
        Self::RuntimeError(Box::new(err))
    }
}

impl From<rand::Error> for AppError {
    fn from(err: rand::Error) -> Self {
        Self::RuntimeError(Box::new(err))
    }
}

/// Result type for this app.
type AppResult<T> = Result<T, AppError>;

/// Parse an argument into the appropriate type.
fn parse_arg<F>(arg: Option<&str>) -> AppResult<Option<F>>
where
    F: FromStr,
    F::Err: Error,
{
    match arg.map(|s| s.parse()) {
        Some(Ok(f)) => Ok(Some(f)),
        Some(Err(e)) => Err(AppError::invalid_value(&e.to_string())),
        None => Ok(None),
    }
}

/// The parsed command line arguments.
#[derive(Debug)]
struct Args {
    source: SourceArg,
    order: OrderArg,
    stripe: bool,
    frontier: FrontierArg,
    space: ColorSpaceArg,
    width: Option<u32>,
    height: Option<u32>,
    x0: Option<u32>,
    y0: Option<u32>,
    animate: bool,
    output: PathBuf,
    seed: u64,
}

impl Args {
    fn parse() -> AppResult<Self> {
        let args = clap_app!((crate_name!()) =>
            (version: crate_version!())
            (author: crate_authors!())
            (@setting ColoredHelp)
            (@setting DeriveDisplayOrder)
            (@setting UnifiedHelpMessage)
            (@group source =>
                (@arg DEPTH: -b --("bit-depth") +takes_value "Use all DEPTH-bit colors")
                (@arg INPUT: -i --input +takes_value "Use colors from the INPUT image")
            )
            (@group order =>
                (@arg HUE: -s --hue-sort "Sort colors by hue [default]")
                (@arg RANDOM: -r --random "Randomize colors")
                (@arg MORTON: -M --morton "Place colors in Morton order (Z-order)")
                (@arg HILBERT: -H --hilbert "Place colors in Hilbert curve order")
            )
            (@group stripe =>
                (@arg STRIPE: -t --stripe "Reduce artifacts by iterating through the colors in multiple stripes [default]")
                (@arg NOSTRIPE: -T --("no-stripe") "Don't stripe")
            )
            (@group frontier =>
                (@arg MODE: -l --selection +takes_value possible_value[min mean] "Specify the selection mode")
                (@arg TARGET: -g --target +takes_value "Place colors on the closest pixels of the TARGET image")
            )
            (@arg SPACE: -c --("color-space") default_value("Lab") possible_value[RGB Lab Luv] "Use the given color space")
            (@arg WIDTH: -w --width +takes_value "The width of the generated image")
            (@arg HEIGHT: -h --height +takes_value "The height of the generated image")
            (@arg X: -x +takes_value "The x coordinate of the first pixel")
            (@arg Y: -y +takes_value "The y coordinate of the first pixel")
            (@arg ANIMATE: -a --animate "Generate frames of an animation")
            (@arg PATH: -o --output default_value("kd-forest.png") "Save the image to PATH")
            (@arg SEED: -e --seed default_value("0") "Seed the random number generator")
        )
        .get_matches_safe()?;

        let source = if let Some(input) = args.value_of("INPUT") {
            SourceArg::Image(PathBuf::from(input))
        } else {
            let arg = args.value_of("DEPTH");
            let depths: Vec<_> = arg
                .iter()
                .map(|s| s.split(','))
                .flatten()
                .map(|n| n.parse().ok())
                .collect();

            let (r, g, b) = match depths.as_slice() {
                [] => (8, 8, 8),

                // Allocate bits from most to least perceptually important
                [Some(d)] => ((d + 1) / 3, (d + 2) / 3, d / 3),

                [Some(r), Some(g), Some(b)] => (*r, *g, *b),

                _ => {
                    return Err(AppError::invalid_value(
                        &format!("invalid bit depth {}", arg.unwrap()),
                    ));
                }
            };

            if r > 8 || g > 8 || b > 8 {
                return Err(AppError::invalid_value(
                    &format!("bit depth of {} is too deep!", arg.unwrap()),
                ));
            }

            SourceArg::AllRgb(r, g, b)
        };

        let order = if args.is_present("RANDOM") {
            OrderArg::Random
        } else if args.is_present("MORTON") {
            OrderArg::Morton
        } else if args.is_present("HILBERT") {
            OrderArg::Hilbert
        } else {
            OrderArg::HueSort
        };

        let stripe = !args.is_present("NOSTRIPE") && order != OrderArg::Random;

        let frontier = if let Some(target) = args.value_of("TARGET") {
            FrontierArg::Image(PathBuf::from(target))
        } else {
            match args.value_of("MODE") {
                Some("min") | None => FrontierArg::Min,
                Some("mean") => FrontierArg::Mean,
                _ => unreachable!(),
            }
        };

        let space = match args.value_of("SPACE").unwrap() {
            "RGB" => ColorSpaceArg::Rgb,
            "Lab" => ColorSpaceArg::Lab,
            "Luv" => ColorSpaceArg::Luv,
            _ => unreachable!(),
        };

        let width = parse_arg(args.value_of("WIDTH"))?;
        let height = parse_arg(args.value_of("HEIGHT"))?;
        let x0 = parse_arg(args.value_of("X"))?;
        let y0 = parse_arg(args.value_of("Y"))?;

        let animate = args.is_present("ANIMATE");

        let output = args.value_of("PATH")
            .map(PathBuf::from)
            .unwrap();

        let seed = parse_arg(args.value_of("SEED"))?.unwrap_or(0);

        Ok(Self {
            source,
            order,
            stripe,
            frontier,
            space,
            width,
            height,
            x0,
            y0,
            animate,
            output,
            seed,
        })
    }
}

/// The kd-forest application itself.
#[derive(Debug)]
struct App {
    args: Args,
    rng: Pcg64,
    width: Option<u32>,
    height: Option<u32>,
    start_time: Instant,
}

impl App {
    /// Make the App.
    fn new(args: Args) -> Self {
        let rng = Pcg64::seed_from_u64(args.seed);
        let width = args.width;
        let height = args.height;
        let start_time = Instant::now();

        Self {
            args,
            rng,
            width,
            height,
            start_time,
        }
    }

    fn run(&mut self) -> AppResult<()> {
        let colors = match self.args.source {
            SourceArg::AllRgb(r, g, b) => {
                let total = r + g + b;
                self.width.get_or_insert(1u32 << ((total + 1) / 2));
                self.height.get_or_insert(1u32 << (total / 2));
                self.get_colors(AllColors::new(r, g, b))
            }
            SourceArg::Image(ref path) => {
                let img = image::open(path)?.into_rgb8();
                self.width.get_or_insert(img.width());
                self.height.get_or_insert(img.height());
                self.get_colors(ImageColors::from(img))
            }
        };

        match self.args.space {
            ColorSpaceArg::Rgb => self.paint::<RgbSpace>(colors),
            ColorSpaceArg::Lab => self.paint::<LabSpace>(colors),
            ColorSpaceArg::Luv => self.paint::<LuvSpace>(colors),
        }
    }

    fn get_colors<S: ColorSource>(&mut self, source: S) -> Vec<Rgb8> {
        let colors = match self.args.order {
            OrderArg::HueSort => order::hue_sorted(source),
            OrderArg::Random => order::shuffled(source, &mut self.rng),
            OrderArg::Morton => order::morton(source),
            OrderArg::Hilbert => order::hilbert(source),
        };

        if self.args.stripe {
            order::striped(colors)
        } else {
            colors
        }
    }

    fn paint<C: ColorSpace>(&mut self, colors: Vec<Rgb8>) -> AppResult<()>
    where
        C::Value: PartialOrd<C::Distance>,
    {
        let width = self.width.unwrap();
        let height = self.height.unwrap();
        let x0 = self.args.x0.unwrap_or(width / 2);
        let y0 = self.args.y0.unwrap_or(height / 2);

        if x0 >= width || y0 >= height {
            return Err(AppError::invalid_value(
                &format!("Initial pixel ({}, {}) is out of bounds ({}, {})", x0, y0, width, height),
            ));
        }

        match &self.args.frontier {
            FrontierArg::Image(ref path) => {
                let img = image::open(path)?.into_rgb8();
                self.paint_on(colors, ImageFrontier::<C>::new(&img))
            }
            FrontierArg::Min => {
                let rng = Pcg64::from_rng(&mut self.rng)?;
                self.paint_on(colors, MinFrontier::<C, _>::new(rng, width, height, x0, y0))
            }
            FrontierArg::Mean => {
                self.paint_on(colors, MeanFrontier::<C>::new(width, height, x0, y0))
            }
        }
    }

    fn write_frame(image: &RgbaImage) -> AppResult<()> {
        let stdout = io::stdout();
        if stdout.is_terminal() {
            return Err(AppError::invalid_value(
                "Not writing images to your terminal, please pipe the output somewhere"
            ));
        }

        let writer = BufWriter::new(stdout.lock());
        let encoder = PngEncoder::new_with_quality(writer, CompressionType::Rle, FilterType::NoFilter);
        encoder.encode(image, image.width(), image.height(), ColorType::Rgba8)?;

        Ok(())
    }

    fn paint_on<F: Frontier>(&mut self, colors: Vec<Rgb8>, mut frontier: F) -> AppResult<()> {
        let width = frontier.width();
        let height = frontier.height();
        let mut output = RgbaImage::new(width, height);

        let size = cmp::min((width * height) as usize, colors.len());
        eprintln!("Generating a {}x{} image ({} pixels)", width, height, size);

        if self.args.animate {
            Self::write_frame(&output)?;
        }

        let interval = cmp::max(width, height) as usize;

        let mut max_frontier = frontier.len();

        for (i, color) in colors.into_iter().enumerate() {
            let pos = frontier.place(color);
            if pos.is_none() {
                break;
            }

            let (x, y) = pos.unwrap();
            let rgba = Rgba([color[0], color[1], color[2], 255]);
            output.put_pixel(x, y, rgba);

            max_frontier = cmp::max(max_frontier, frontier.len());

            if (i + 1) % interval == 0 {
                if self.args.animate {
                    Self::write_frame(&output)?;
                }

                if i + 1 < size {
                    self.print_progress(i + 1, size, frontier.len())?;
                }
            }
        }

        if self.args.animate && size % interval != 0 {
            Self::write_frame(&output)?;
        }

        self.print_progress(size, size, max_frontier)?;

        if !self.args.animate {
            output.save(&self.args.output)?;
        }

        Ok(())
    }

    fn print_progress(&self, i: usize, size: usize, frontier_len: usize) -> io::Result<()> {
        let mut term = match term::stderr() {
            Some(term) => term,
            None => return Ok(()),
        };

        let progress = 100.0 * (i as f64) / (size as f64);
        let mut rate = (i as f64) / self.start_time.elapsed().as_secs_f64();
        let mut unit = "px/s";

        if rate >= 10_000.0 {
            rate /= 1_000.0;
            unit = "kpx/s";
        }

        if rate >= 10_000.0 {
            rate /= 1_000.0;
            unit = "Mpx/s";
        }

        if rate >= 10_000.0 {
            rate /= 1_000.0;
            unit = "Gpx/s";
        }

        let (frontier_label, newline) = if i == size {
            ("max frontier size", "\n")
        } else {
            ("frontier size", "")
        };

        term.carriage_return()?;
        term.delete_line()?;

        write!(
            term,
            "{:>6.2}%  | {:4.0} {:>5}  | {}: {}{}",
            progress, rate, unit, frontier_label, frontier_len, newline,
        )
    }
}

fn main() {
    let args = match Args::parse() {
        Ok(args) => args,
        Err(e) => e.exit(),
    };

    match App::new(args).run() {
        Ok(_) => {},
        Err(e) => e.exit(),
    }
}
