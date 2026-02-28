use nu_ansi_term::Color;

use crate::{app::AppBuilder, config::env::Env};

const BANNER: &str = r"
            ⌡
         __@▓▄                             ___
   ___,p@▒▒░▓▓L                            ▀▀▀ª    ▄▄▄▄▄
  ⌡▒▒▒▓▒▒░▒▓▓▓▓  ▄█████  ████████_  ██████ ███ª █████████,  ▄████████
_▄▒▓▒▒▓▒▒░▓▓▓▓▓  ▓██▄_   ███~ ~███, ███▀ª~ ███─ ███▓~~▓██N ███▀~~▓██▀
▄▓▒▓▒▒▓▒▓▓▓▓▓▓█   ~▀███  ███_ _███'▐███    ███  ███ª  ▓██N ███▄ _███G
▓▓▒▓▒▒▓▓▓▓▓▓▓▀  ▀█████▀  ███▀████ª ▐███    ███  ███   ▓██N ~▀███▀███ª
▓▓▓▓▓▓▓▓▓▓▀ª`            ███                                ▄▄▄▄▄███
 ▓▀▀▀ⁿª^                 ▀▀▀                                ▀▀▀▀▀▀▀~
";

pub(crate) fn print_banner(app: &AppBuilder) {
    println!("{BANNER}");
    println!(
        "     summer: {}",
        Color::Green.paint(env!("CARGO_PKG_VERSION"))
    );
    let env = match app.env {
        Env::Dev => Color::LightYellow.paint("Dev"),
        Env::Test => Color::LightBlue.paint("Test"),
        Env::Prod => Color::Green.paint("Prod"),
    };
    println!("environment: {env}");
    if cfg!(debug_assertions) {
        println!("compilation: {}", Color::LightRed.paint("Debug"));
    } else {
        println!("compilation: {}", Color::Green.paint("Release"));
    }
}
