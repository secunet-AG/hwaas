// SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
//
// SPDX-License-Identifier: Apache-2.0

use embedded_graphics::{
    mono_font::{ascii::FONT_6X10, MonoTextStyleBuilder},
    pixelcolor::BinaryColor,
    prelude::*,
    text::{Baseline, Text},
};
use rppal::i2c::I2c;
use sh1106::Builder;
use ssd1306::{I2CDisplayInterface, Ssd1306};
use std::fmt::Debug;

use clap::{Parser, ValueEnum};
use sh1106::prelude::GraphicsMode;
use ssd1306::prelude::{DisplayConfig, DisplayRotation, DisplaySize128x64};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct CliArgs {
    /// name to display
    #[arg(short, long)]
    text: String,

    /// if specified logging messages are written to this file (additionally to stdout)
    // #[arg(short, long)]
    // log_file: Option<PathBuf>,

    /// level of verbosity (could be used several times; e.g. '-vv')
    #[arg(short, long, action = clap::ArgAction::Count)]
    verbose: u8,

    #[arg(value_enum, default_value_t = DisplayType::default())]
    display_type: DisplayType,
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum, Default, Debug)]
pub enum DisplayType {
    SSD1306,
    #[default]
    SH1106,
}

fn main() -> Result<(), ()> {
    let args: CliArgs = CliArgs::parse();

    let i2c = I2c::new().unwrap();

    match args.display_type {
        DisplayType::SSD1306 => {
            let interface = I2CDisplayInterface::new(i2c);
            let mut display = Ssd1306::new(interface, DisplaySize128x64, DisplayRotation::Rotate0)
                .into_buffered_graphics_mode();
            display.init().unwrap();
            run(args, &mut display)?;
            display.flush().unwrap();
        }
        DisplayType::SH1106 => {
            let mut display: GraphicsMode<_> = Builder::new().connect_i2c(i2c).into();
            display.init().unwrap();
            run(args, &mut display)?;
            display.flush().unwrap();
        }
    }

    Ok(())
}

fn run<D: DrawTarget<Color = BinaryColor>>(args: CliArgs, display: &mut D) -> Result<(), ()>
where
    <D as embedded_graphics::draw_target::DrawTarget>::Error: Debug,
{
    let text_style = MonoTextStyleBuilder::new()
        .font(&FONT_6X10)
        .text_color(BinaryColor::On)
        .build();

    Text::with_baseline(args.text.as_str(), Point::zero(), text_style, Baseline::Top)
        .draw(display)
        .unwrap();

    Ok(())
}
