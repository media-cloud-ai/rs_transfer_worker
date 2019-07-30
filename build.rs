
use std::process::Command;

fn main() {
  let cmd = Command::new("git")
    .args(&["describe", "--tag", "--always"])
    .output()
    .unwrap();
  assert!(cmd.status.success());
  let ver = std::str::from_utf8(&cmd.stdout[..]).unwrap().trim();
  println!("cargo:rustc-env={}={}", "VERSION", ver);
  println!("cargo:rerun-if-changed=(nonexistentfile)");
}
