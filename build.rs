
use std::env;
use std::process::Command;

fn main() {
  let version = 
    if let Ok(version) = env::var("IMAGE_TAG") {
      version
    } else {
      let cmd = Command::new("git")
        .args(&["describe", "--tag", "--always"])
        .output()
        .unwrap();
      assert!(cmd.status.success());
      std::str::from_utf8(&cmd.stdout[..]).unwrap().trim().to_string()
    };

  println!("cargo:rustc-env={}={}", "VERSION", version);
  println!("cargo:rerun-if-changed=(nonexistentfile)");
}
