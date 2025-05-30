use std::env;
use std::fs::{self, File};
use std::io::{self, Write};

fn main() {
    UserApp::generate_asm();
    println!("cargo::rerun-if-changed=src/link_apps.rs");
    println!("cargo::rerun-if-changed=../user/src/bin");
    println!("cargo::rerun-if-changed=../user/target/riscv64gc-unknown-none-elf/release");
}

struct UserApp;

impl UserApp {
    const OUTPUT: &str = "src/link_apps.S";
    const SRC_DIR: &str = "../user/src/bin";
    const SRC_EXTENSION: &str = ".rs";
    const BIN_DIR: &str = "../user/target/riscv64gc-unknown-none-elf/release";

    fn generate_asm() {
        let mut names = Self::get_app_names();
        names.sort();

        let mut dst = File::create(Self::OUTPUT).unwrap();
        Self::write_app_asms(&mut dst, names).unwrap();
    }

    fn get_app_names() -> Vec<String> {
        let include_test = env::var("TEST").is_ok_and(|s| s == "1");

        fs::read_dir(Self::SRC_DIR)
            .unwrap()
            .map(|entry| entry.unwrap().file_name().into_string().unwrap())
            .map(|file_name| {
                file_name
                    .strip_suffix(Self::SRC_EXTENSION)
                    .unwrap()
                    .to_string()
            })
            .filter(|name| {
                if include_test {
                    true
                } else {
                    !name.starts_with("test_")
                }
            })
            .collect::<Vec<_>>()
    }

    fn write_app_asms(dst: &mut File, app_names: Vec<String>) -> io::Result<()> {
        let total_apps = app_names.len();

        // Write the top summary part
        writeln!(
            dst,
            r#"# This file is generated by the build.rs

    .align 3
    .section .data
    .global _num_apps
_num_apps:
    .quad {total_apps}"#,
        )?;

        for i in 0..total_apps {
            writeln!(
                dst,
                r#"    .quad app_{i}_name
    .quad app_{i}_start
    .quad app_{i}_end"#
            )?;
        }

        // Write the per-app part
        for i in 0..total_apps {
            writeln!(
                dst,
                r#"
    .section .data
    .global app_{i}_name
    .global app_{i}_start
    .global app_{i}_end
app_{i}_name:
    .ascii "{app_name_bytes}"
app_{i}_start:
    .incbin "{bin_dir}/{app_name}.bin"
app_{i}_end:"#,
                app_name_bytes = app_names[i],
                bin_dir = Self::BIN_DIR,
                app_name = app_names[i]
            )?;
        }

        Ok(())
    }
}
