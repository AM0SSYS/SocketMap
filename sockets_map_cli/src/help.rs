//! This module contains help messages and help display functions.

use sockets_map::help;
use std::fmt::Write;

pub struct HelpMessages {
    linux: String,
    windows: String,
    unknown_remote: String,
    csv: String,
    summary: String,
}

struct MDCatSettings<'a> {
    settings: mdcat::Settings,
    env: mdcat::Environment,
    parser: pulldown_cmark::Parser<'a, 'a>,
}

/// Init mdcat
fn init_mdcat(md_text: &str) -> MDCatSettings<'_> {
    let terminal_capabilities = mdcat::TerminalCapabilities::detect();
    let terminal_size = mdcat::TerminalSize::detect().expect("unable to get terminal size");
    let resource_access = mdcat::ResourceAccess::LocalOnly;
    let syntax_set = syntect::parsing::SyntaxSet::load_defaults_newlines();
    let settings = mdcat::Settings {
        terminal_capabilities,
        terminal_size,
        resource_access,
        syntax_set,
    };
    let env = mdcat::Environment::for_local_directory(
        &std::env::current_dir().expect("unable to get current directory"),
    )
    .expect("unable to make mdcat env");
    let parser = pulldown_cmark::Parser::new_ext(md_text, pulldown_cmark::Options::all());
    MDCatSettings {
        settings,
        env,
        parser,
    }
}

impl HelpMessages {
    /// Print a Markdown formatted help message
    fn print_stdout(&self, md_text: &str) {
        let mdcat_settings = init_mdcat(md_text);
        mdcat::push_tty(
            &mdcat_settings.settings,
            &mdcat_settings.env,
            &mut std::io::stdout(),
            mdcat_settings.parser,
        )
        .expect("unable to write Markdown formatted text to output");
    }

    /// Print help message for Linux
    pub fn print_linux(&self) {
        self.print_stdout(&self.linux);
    }

    /// Print help message for Windows
    pub fn print_windows(&self) {
        self.print_stdout(&self.windows);
    }

    /// Print help message for Remote Unknown
    pub fn print_unknown_remote(&self) {
        self.print_stdout(&self.unknown_remote);
    }

    /// Print help message for Remote Unknown
    pub fn print_csv(&self) {
        self.print_stdout(&self.csv);
    }

    /// Print summary
    pub fn print_summary(&self) {
        self.print_stdout(&self.summary);
    }

    // Print all
    pub fn print_all(&self) {
        // Initialize minus pager
        let mut output = minus::Pager::new();

        // Write all to pager
        let mut text = String::new();
        text.push_str(&self.summary);
        text.push('\n');
        text.push_str(&self.linux);
        text.push('\n');
        text.push_str(&self.windows);
        text.push('\n');
        text.push_str(&self.unknown_remote);
        text.push('\n');
        text.push_str(&self.csv);
        let mdcat_settings = init_mdcat(&text);
        let mut buf = std::io::BufWriter::new(Vec::new());
        mdcat::push_tty(
            &mdcat_settings.settings,
            &mdcat_settings.env,
            &mut buf,
            mdcat_settings.parser,
        )
        .expect("unable to write Markdown formatted text to output");
        let bytes = buf.into_inner().unwrap();
        let string_buffer =
            String::from_utf8(bytes).expect("unable to make String from utf8 for pager");
        writeln!(output, "{string_buffer}").expect("unable to write to pager");

        // Run the pager
        minus::page_all(output).expect("unable to run the pager");
    }
}

impl Default for HelpMessages {
    fn default() -> Self {
        Self {
            linux: help::LINUX_HELP.to_string(),
            windows: help::WINDOWS_HELP.to_string(),
            unknown_remote: help::UNKNOWN_REMOTE_HELP.to_string(),
            csv: help::CSV_HELP.to_string(),
            summary: help::SUMMARY_HELP.to_string(),
        }
    }
}
