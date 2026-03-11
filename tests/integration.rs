use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use tempfile::TempDir;

fn mojo() -> Command {
    Command::cargo_bin("mojo").unwrap()
}

// ── mojo new ────────────────────────────────────────────

#[test]
fn new_bin_cpp() {
    let tmp = TempDir::new().unwrap();
    mojo()
        .args(["new", "hello"])
        .current_dir(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("Created"));

    assert!(tmp.path().join("hello/Mojo.toml").exists());
    assert!(tmp.path().join("hello/src/main.cpp").exists());
    assert!(tmp.path().join("hello/.gitignore").exists());
}

#[test]
fn new_lib_c() {
    let tmp = TempDir::new().unwrap();
    mojo()
        .args(["new", "mylib", "--lang=c", "--lib"])
        .current_dir(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("Created"));

    assert!(tmp.path().join("mylib/Mojo.toml").exists());
    assert!(tmp.path().join("mylib/include/mylib.h").exists());
    assert!(tmp.path().join("mylib/src/mylib.c").exists());
}

#[test]
fn new_invalid_name() {
    let tmp = TempDir::new().unwrap();
    mojo()
        .args(["new", "../bad"])
        .current_dir(tmp.path())
        .assert()
        .failure()
        .stderr(predicate::str::contains("invalid characters"));
}

#[test]
fn new_invalid_lang() {
    let tmp = TempDir::new().unwrap();
    mojo()
        .args(["new", "foo", "--lang=rust"])
        .current_dir(tmp.path())
        .assert()
        .failure();
}

// ── mojo init ───────────────────────────────────────────

#[test]
fn init_in_existing_dir() {
    let tmp = TempDir::new().unwrap();
    let project_dir = tmp.path().join("myproject");
    fs::create_dir(&project_dir).unwrap();

    mojo()
        .args(["init"])
        .current_dir(&project_dir)
        .assert()
        .success()
        .stdout(predicate::str::contains("Initialized"));

    assert!(project_dir.join("Mojo.toml").exists());
    assert!(project_dir.join("src/main.cpp").exists());
}

// ── mojo build ──────────────────────────────────────────

#[test]
fn build_debug() {
    let tmp = TempDir::new().unwrap();
    mojo()
        .args(["new", "hello"])
        .current_dir(tmp.path())
        .assert()
        .success();

    mojo()
        .args(["build"])
        .current_dir(tmp.path().join("hello"))
        .assert()
        .success()
        .stdout(predicate::str::contains("Compiling"))
        .stdout(predicate::str::contains("Finished"));

    assert!(tmp.path().join("hello/build/debug/hello").exists());
}

#[test]
fn build_release() {
    let tmp = TempDir::new().unwrap();
    mojo()
        .args(["new", "hello"])
        .current_dir(tmp.path())
        .assert()
        .success();

    mojo()
        .args(["build", "--release"])
        .current_dir(tmp.path().join("hello"))
        .assert()
        .success()
        .stdout(predicate::str::contains("release"));

    assert!(tmp.path().join("hello/build/release/hello").exists());
}

// ── mojo run ────────────────────────────────────────────

#[test]
fn run_executes_binary() {
    let tmp = TempDir::new().unwrap();
    mojo()
        .args(["new", "hello"])
        .current_dir(tmp.path())
        .assert()
        .success();

    mojo()
        .args(["run"])
        .current_dir(tmp.path().join("hello"))
        .assert()
        .success()
        .stdout(predicate::str::contains("Hello, world!"));
}

#[test]
fn run_errors_on_lib() {
    let tmp = TempDir::new().unwrap();
    mojo()
        .args(["new", "mylib", "--lib"])
        .current_dir(tmp.path())
        .assert()
        .success();

    mojo()
        .args(["run"])
        .current_dir(tmp.path().join("mylib"))
        .assert()
        .failure()
        .stderr(predicate::str::contains("cannot run a library project"));
}

// ── mojo clean ──────────────────────────────────────────

#[test]
fn clean_removes_build_dir() {
    let tmp = TempDir::new().unwrap();
    mojo()
        .args(["new", "hello"])
        .current_dir(tmp.path())
        .assert()
        .success();

    mojo()
        .args(["build"])
        .current_dir(tmp.path().join("hello"))
        .assert()
        .success();

    assert!(tmp.path().join("hello/build").exists());

    mojo()
        .args(["clean"])
        .current_dir(tmp.path().join("hello"))
        .assert()
        .success()
        .stdout(predicate::str::contains("Removed"));

    assert!(!tmp.path().join("hello/build").exists());
}

// ── path dependency ─────────────────────────────────────

#[test]
fn path_dependency() {
    let tmp = TempDir::new().unwrap();

    // Create a library
    mojo()
        .args(["new", "mylib", "--lib", "--lang=c"])
        .current_dir(tmp.path())
        .assert()
        .success();

    // Create a binary
    mojo()
        .args(["new", "app", "--lang=c"])
        .current_dir(tmp.path())
        .assert()
        .success();

    // Update app's Mojo.toml to depend on mylib
    let config = format!(
        r#"[package]
name = "app"
version = "0.1.0"
lang = "c"
std = "c11"

[build]
compiler = "auto"

[dependencies]
mylib = {{ path = "../mylib" }}
"#
    );
    fs::write(tmp.path().join("app/Mojo.toml"), config).unwrap();

    // Update app's main.c to use mylib
    let main_c = r#"#include <stdio.h>
#include "mylib.h"

int main(void) {
    printf("3 + 4 = %d\n", mylib_add(3, 4));
    return 0;
}
"#;
    fs::write(tmp.path().join("app/src/main.c"), main_c).unwrap();

    mojo()
        .args(["run"])
        .current_dir(tmp.path().join("app"))
        .assert()
        .success()
        .stdout(predicate::str::contains("3 + 4 = 7"));
}

// ── incremental build ───────────────────────────────────

#[test]
fn incremental_build() {
    let tmp = TempDir::new().unwrap();
    mojo()
        .args(["new", "hello"])
        .current_dir(tmp.path())
        .assert()
        .success();

    // First build
    mojo()
        .args(["build"])
        .current_dir(tmp.path().join("hello"))
        .assert()
        .success()
        .stdout(predicate::str::contains("Compiling"));

    // Second build — should still show "Finished" but skip compilation
    let output = mojo()
        .args(["build"])
        .current_dir(tmp.path().join("hello"))
        .assert()
        .success();

    // The output should still contain Finished
    output.stdout(predicate::str::contains("Finished"));
}

// ── --verbose flag ──────────────────────────────────────

#[test]
fn verbose_shows_compiler() {
    let tmp = TempDir::new().unwrap();
    mojo()
        .args(["new", "hello"])
        .current_dir(tmp.path())
        .assert()
        .success();

    mojo()
        .args(["-v", "build"])
        .current_dir(tmp.path().join("hello"))
        .assert()
        .success()
        .stdout(predicate::str::contains("Running"));
}

// ── --quiet flag ────────────────────────────────────────

#[test]
fn quiet_suppresses_output() {
    let tmp = TempDir::new().unwrap();
    mojo()
        .args(["new", "hello"])
        .current_dir(tmp.path())
        .assert()
        .success();

    mojo()
        .args(["-q", "build"])
        .current_dir(tmp.path().join("hello"))
        .assert()
        .success()
        .stdout(predicate::str::is_empty());
}

// ── invalid config ──────────────────────────────────────

#[test]
fn invalid_opt_level() {
    let tmp = TempDir::new().unwrap();
    mojo()
        .args(["new", "hello"])
        .current_dir(tmp.path())
        .assert()
        .success();

    let config = r#"[package]
name = "hello"
version = "0.1.0"
lang = "c++"
std = "c++17"

[profile.debug]
opt_level = "fast"
"#;
    fs::write(tmp.path().join("hello/Mojo.toml"), config).unwrap();

    mojo()
        .args(["build"])
        .current_dir(tmp.path().join("hello"))
        .assert()
        .failure()
        .stderr(predicate::str::contains("invalid opt_level"));
}

#[test]
fn empty_version() {
    let tmp = TempDir::new().unwrap();
    mojo()
        .args(["new", "hello"])
        .current_dir(tmp.path())
        .assert()
        .success();

    let config = r#"[package]
name = "hello"
version = ""
lang = "c++"
"#;
    fs::write(tmp.path().join("hello/Mojo.toml"), config).unwrap();

    mojo()
        .args(["build"])
        .current_dir(tmp.path().join("hello"))
        .assert()
        .failure()
        .stderr(predicate::str::contains("version cannot be empty"));
}

// ── shared library ──────────────────────────────────────

#[test]
fn shared_library() {
    let tmp = TempDir::new().unwrap();
    mojo()
        .args(["new", "mylib", "--lib", "--lang=c"])
        .current_dir(tmp.path())
        .assert()
        .success();

    // Change to shared lib
    let config = r#"[package]
name = "mylib"
version = "0.1.0"
lang = "c"
std = "c11"
type = "lib"
lib-type = "shared"
"#;
    fs::write(tmp.path().join("mylib/Mojo.toml"), config).unwrap();

    mojo()
        .args(["build"])
        .current_dir(tmp.path().join("mylib"))
        .assert()
        .success()
        .stdout(predicate::str::contains("Linking"));

    // Check for shared lib
    if cfg!(target_os = "macos") {
        assert!(tmp.path().join("mylib/build/debug/libmylib.dylib").exists());
    } else {
        assert!(tmp.path().join("mylib/build/debug/libmylib.so").exists());
    }
}

// ── custom cflags ───────────────────────────────────────

#[test]
fn custom_cflags_define() {
    let tmp = TempDir::new().unwrap();
    mojo()
        .args(["new", "hello", "--lang=c"])
        .current_dir(tmp.path())
        .assert()
        .success();

    let config = r#"[package]
name = "hello"
version = "0.1.0"
lang = "c"
std = "c11"

[build]
compiler = "auto"
cflags = ["-DTEST_MACRO"]
"#;
    fs::write(tmp.path().join("hello/Mojo.toml"), config).unwrap();

    let main_c = r#"#include <stdio.h>

int main(void) {
#ifdef TEST_MACRO
    printf("macro defined\n");
#else
    printf("macro not defined\n");
#endif
    return 0;
}
"#;
    fs::write(tmp.path().join("hello/src/main.c"), main_c).unwrap();

    mojo()
        .args(["run"])
        .current_dir(tmp.path().join("hello"))
        .assert()
        .success()
        .stdout(predicate::str::contains("macro defined"));
}

// ── parallel build with -j ──────────────────────────────

#[test]
fn parallel_build_j_flag() {
    let tmp = TempDir::new().unwrap();
    mojo()
        .args(["new", "hello", "--lang=c"])
        .current_dir(tmp.path())
        .assert()
        .success();

    // Add a second source file
    let extra = r#"// extra.c
int extra_func(void) { return 42; }
"#;
    fs::write(tmp.path().join("hello/src/extra.c"), extra).unwrap();

    // Update main to reference it
    let main_c = r#"#include <stdio.h>
extern int extra_func(void);
int main(void) {
    printf("extra = %d\n", extra_func());
    return 0;
}
"#;
    fs::write(tmp.path().join("hello/src/main.c"), main_c).unwrap();

    mojo()
        .args(["run", "-j", "4"])
        .current_dir(tmp.path().join("hello"))
        .assert()
        .success()
        .stdout(predicate::str::contains("extra = 42"));
}

// ── lock file ───────────────────────────────────────────

#[test]
fn lock_file_created_on_build() {
    let tmp = TempDir::new().unwrap();
    mojo()
        .args(["new", "hello"])
        .current_dir(tmp.path())
        .assert()
        .success();

    mojo()
        .args(["build"])
        .current_dir(tmp.path().join("hello"))
        .assert()
        .success();

    // Lock file should be created (even if empty deps)
    assert!(tmp.path().join("hello/Mojo.lock").exists());
}

// ── mojo update ─────────────────────────────────────────

#[test]
fn update_regenerates_lock() {
    let tmp = TempDir::new().unwrap();
    mojo()
        .args(["new", "hello"])
        .current_dir(tmp.path())
        .assert()
        .success();

    // Build first to create lock
    mojo()
        .args(["build"])
        .current_dir(tmp.path().join("hello"))
        .assert()
        .success();

    assert!(tmp.path().join("hello/Mojo.lock").exists());

    // Update should recreate it
    mojo()
        .args(["update"])
        .current_dir(tmp.path().join("hello"))
        .assert()
        .success()
        .stdout(predicate::str::contains("Updated"));

    assert!(tmp.path().join("hello/Mojo.lock").exists());
}

// ── framework templates ────────────────────────────────

#[test]
fn new_with_qt() {
    let tmp = TempDir::new().unwrap();
    mojo()
        .args(["new", "myqtapp", "--qt"])
        .current_dir(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("Qt support"));

    let root = tmp.path().join("myqtapp");
    assert!(root.join("Mojo.toml").exists());
    assert!(root.join("src/main.cpp").exists());
    assert!(root.join("include/mainwindow.hpp").exists());

    let toml = fs::read_to_string(root.join("Mojo.toml")).unwrap();
    assert!(toml.contains("Qt5Widgets"));
    assert!(toml.contains("lang = \"c++\""));
}

#[test]
fn new_with_gtk() {
    let tmp = TempDir::new().unwrap();
    mojo()
        .args(["new", "mygtkapp", "--gtk"])
        .current_dir(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("GTK support"));

    let root = tmp.path().join("mygtkapp");
    assert!(root.join("Mojo.toml").exists());
    assert!(root.join("src/main.c").exists());

    let toml = fs::read_to_string(root.join("Mojo.toml")).unwrap();
    assert!(toml.contains("gtk+-3.0"));
    assert!(toml.contains("lang = \"c\""));

    let main = fs::read_to_string(root.join("src/main.c")).unwrap();
    assert!(main.contains("gtk/gtk.h"));
}

#[test]
fn new_with_libcurl() {
    let tmp = TempDir::new().unwrap();
    mojo()
        .args(["new", "curlapp", "--libcurl"])
        .current_dir(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("libcurl support"));

    let root = tmp.path().join("curlapp");
    assert!(root.join("src/main.c").exists());

    let toml = fs::read_to_string(root.join("Mojo.toml")).unwrap();
    assert!(toml.contains("\"curl\""));

    let main = fs::read_to_string(root.join("src/main.c")).unwrap();
    assert!(main.contains("curl/curl.h"));
}

#[test]
fn new_with_grpc() {
    let tmp = TempDir::new().unwrap();
    mojo()
        .args(["new", "grpcapp", "--grpc"])
        .current_dir(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("gRPC support"));

    let root = tmp.path().join("grpcapp");
    assert!(root.join("src/main.cpp").exists());
    assert!(root.join("proto/hello.proto").exists());

    let toml = fs::read_to_string(root.join("Mojo.toml")).unwrap();
    assert!(toml.contains("grpc++"));

    let proto = fs::read_to_string(root.join("proto/hello.proto")).unwrap();
    assert!(proto.contains("service Greeter"));
}

#[test]
fn new_with_gtest() {
    let tmp = TempDir::new().unwrap();
    mojo()
        .args(["new", "testapp", "--gtest"])
        .current_dir(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("Google Test support"));

    let root = tmp.path().join("testapp");
    assert!(root.join("src/main.cpp").exists());

    let toml = fs::read_to_string(root.join("Mojo.toml")).unwrap();
    assert!(toml.contains("gtest"));

    let main = fs::read_to_string(root.join("src/main.cpp")).unwrap();
    assert!(main.contains("gtest/gtest.h"));
    assert!(main.contains("RUN_ALL_TESTS"));
}

#[test]
fn new_with_boost() {
    let tmp = TempDir::new().unwrap();
    mojo()
        .args(["new", "boostapp", "--boost"])
        .current_dir(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("Boost support"));

    let root = tmp.path().join("boostapp");
    assert!(root.join("src/main.cpp").exists());

    let toml = fs::read_to_string(root.join("Mojo.toml")).unwrap();
    assert!(toml.contains("boost_filesystem"));

    let main = fs::read_to_string(root.join("src/main.cpp")).unwrap();
    assert!(main.contains("boost/filesystem.hpp"));
}

#[test]
fn new_with_freertos() {
    let tmp = TempDir::new().unwrap();
    mojo()
        .args(["new", "rtosapp", "--freertos"])
        .current_dir(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("FreeRTOS support"));

    let root = tmp.path().join("rtosapp");
    assert!(root.join("src/main.c").exists());
    assert!(root.join("include/FreeRTOSConfig.h").exists());

    let toml = fs::read_to_string(root.join("Mojo.toml")).unwrap();
    assert!(toml.contains("FreeRTOS-Kernel"));
    assert!(toml.contains("lang = \"c\""));

    let config = fs::read_to_string(root.join("include/FreeRTOSConfig.h")).unwrap();
    assert!(config.contains("configUSE_PREEMPTION"));
}

#[test]
fn new_with_zephyr() {
    let tmp = TempDir::new().unwrap();
    mojo()
        .args(["new", "zephyrapp", "--zephyr"])
        .current_dir(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("Zephyr support"));

    let root = tmp.path().join("zephyrapp");
    assert!(root.join("src/main.c").exists());
    assert!(root.join("prj.conf").exists());
    assert!(root.join("CMakeLists.txt").exists());

    let toml = fs::read_to_string(root.join("Mojo.toml")).unwrap();
    assert!(toml.contains("lang = \"c\""));

    let conf = fs::read_to_string(root.join("prj.conf")).unwrap();
    assert!(conf.contains("CONFIG_PRINTK"));
}

#[test]
fn new_framework_mutual_exclusion() {
    let tmp = TempDir::new().unwrap();
    mojo()
        .args(["new", "badapp", "--qt", "--gtk"])
        .current_dir(tmp.path())
        .assert()
        .failure();
}

#[test]
fn init_with_boost() {
    let tmp = TempDir::new().unwrap();
    let project_dir = tmp.path().join("boostinit");
    fs::create_dir(&project_dir).unwrap();

    mojo()
        .args(["init", "--boost"])
        .current_dir(&project_dir)
        .assert()
        .success()
        .stdout(predicate::str::contains("Boost support"));

    assert!(project_dir.join("Mojo.toml").exists());
    assert!(project_dir.join("src/main.cpp").exists());

    let toml = fs::read_to_string(project_dir.join("Mojo.toml")).unwrap();
    assert!(toml.contains("boost_filesystem"));
}

#[test]
fn new_framework_creates_test_file() {
    let tmp = TempDir::new().unwrap();
    mojo()
        .args(["new", "fwtest", "--gtest"])
        .current_dir(tmp.path())
        .assert()
        .success();

    assert!(tmp.path().join("fwtest/tests/test_basic.cpp").exists());
}

#[test]
fn new_framework_creates_gitignore() {
    let tmp = TempDir::new().unwrap();
    mojo()
        .args(["new", "fwgit", "--libcurl"])
        .current_dir(tmp.path())
        .assert()
        .success();

    assert!(tmp.path().join("fwgit/.gitignore").exists());
    let gi = fs::read_to_string(tmp.path().join("fwgit/.gitignore")).unwrap();
    assert!(gi.contains("/build/"));
}

// ── serial build (j1) ───────────────────────────────────

#[test]
fn serial_build_j1() {
    let tmp = TempDir::new().unwrap();
    mojo()
        .args(["new", "hello"])
        .current_dir(tmp.path())
        .assert()
        .success();

    mojo()
        .args(["build", "-j", "1"])
        .current_dir(tmp.path().join("hello"))
        .assert()
        .success()
        .stdout(predicate::str::contains("Finished"));
}
