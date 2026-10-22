// Force build script rerun
use std::env;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::fs;

fn hack_portfile(vcpkg_dir: &Path) {
    let portfile = vcpkg_dir.join("ports").join("ffmpeg").join("portfile.cmake");
    
    // Always start with a clean portfile from git
    let _ = Command::new("git")
        .args(["checkout", "--", "ports/ffmpeg/portfile.cmake"])
        .current_dir(vcpkg_dir)
        .status();

    let content = fs::read_to_string(&portfile).unwrap();
    
    let minimal_flags = "--disable-everything --enable-decoder=vp8,vp9,av1,libdav1d,libaom_av1,av1_cuvid,av1_qsv,aac,opus --enable-encoder=libvpx_vp8,libvpx_vp9,libaom_av1,av1_nvenc,hevc_nvenc,h264_nvenc,av1_amf,hevc_amf,h264_amf,av1_qsv,aac,opus --enable-parser=vp8,vp9,av1,aac,opus --enable-demuxer=matroska,webm,mov,mp4,m4a,3gp,3g2,mj2 --enable-muxer=webm,matroska,mp4,mov,ipod --enable-protocol=file";
    
    let new_content = content.replace(
        "set(OPTIONS \"--enable-pic",
        &format!("set(OPTIONS \"{} --enable-pic", minimal_flags)
    );
    fs::write(&portfile, new_content).unwrap();
}

fn run_command_streaming(cmd_program: &Path, cmd_args: &[&str], current_dir: &Path) -> std::process::ExitStatus {
    use std::io::{BufRead, BufReader};
    use std::process::Stdio;

    println!("cargo:warning=Running: {} {}", cmd_program.display(), cmd_args.join(" "));

    let mut child = Command::new(cmd_program)
        .args(cmd_args)
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .current_dir(current_dir)
        .spawn()
        .expect("Failed to spawn process");

    if let Some(stdout) = child.stdout.take() {
        let reader = BufReader::new(stdout);
        for l in reader.lines().map_while(Result::ok) {
            println!("cargo:warning={}", l);
        }
    }

    child.wait().expect("Failed to wait on child process")
}

fn main() {
    // Ensure vcpkg binary cache directory exists if specified
    if let Ok(cache_dir) = env::var("VCPKG_DEFAULT_BINARY_CACHE") {
        let _ = fs::create_dir_all(cache_dir);
    }

    // Auto-detect LLVM / libclang on Windows if not explicitly set
    if env::var("LIBCLANG_PATH").is_err() {
        for candidate in &[
            r"C:\Program Files\LLVM\bin",
            r"C:\Program Files (x86)\LLVM\bin",
            r"C:\msys64\clang64\bin",
            r"C:\msys64\mingw64\bin",
        ] {
            let candidate_path = Path::new(candidate);
            if candidate_path.join("libclang.dll").exists() {
                unsafe {
                    env::set_var("LIBCLANG_PATH", candidate_path);
                }
                break;
            }
        }
    }

    println!("cargo:rerun-if-changed=src/ffi/wrapper.hpp");
    
    let target_os = env::var("CARGO_CFG_TARGET_OS").unwrap();
    let target_arch = env::var("CARGO_CFG_TARGET_ARCH").unwrap();

    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    
    // 1. Setup Adobe SDK (Find Windows or Mac SDK zip file or existing directory)
    let sdk_headers = if let Ok(custom_dir) = env::var("ADOBE_SDK_DIR") {
        PathBuf::from(custom_dir)
    } else {
        let manifest_sdk = PathBuf::from(&manifest_dir).join("sdk");
        let direct_headers = manifest_sdk.join("Premiere Pro 26.0 C++ SDK").join("Examples").join("Headers");
        if direct_headers.exists() {
            direct_headers
        } else {
            let adobe_sdk_dir = out_dir.join("Adobe_Premiere_Pro_SDK");
            let extracted_headers = adobe_sdk_dir.join("Premiere Pro 26.0 C++ SDK").join("Examples").join("Headers");
            if !extracted_headers.exists() {
                let mut zip_path = manifest_sdk.join("Premiere Pro 26.0 C++ SDK Windows.zip");
                if !zip_path.exists() {
                    zip_path = manifest_sdk.join("Premiere Pro 26.0 C++ SDK Mac.zip");
                }
                
                fs::create_dir_all(&adobe_sdk_dir).unwrap();
                
                let file = std::fs::File::open(&zip_path).expect("Failed to open Adobe SDK zip (Neither Windows nor Mac zip found)");
                let mut archive = zip::ZipArchive::new(file).expect("Failed to parse zip archive");
                archive.extract(&adobe_sdk_dir).expect("Failed to extract Adobe SDK");
            }
            extracted_headers
        }
    };

    // 2. Generate Premiere Pro Bindings
    let mut builder = bindgen::Builder::default()
        .header("src/ffi/wrapper.hpp")
        .rust_target(bindgen::RustTarget::nightly())
        .clang_arg(format!("-I{}", sdk_headers.display()))
        .clang_arg("-x")
        .clang_arg("c++")
        .clang_arg("-std=c++14")
        .layout_tests(false)
        .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()));

    // Add clang builtin includes if available
    if let Ok(llvm_path) = env::var("LIBCLANG_PATH") {
        let llvm_lib = PathBuf::from(llvm_path).parent().unwrap().join("lib").join("clang");
        if let Ok(entries) = fs::read_dir(llvm_lib) {
            for entry in entries.flatten() {
                let inc = entry.path().join("include");
                if inc.exists() {
                    builder = builder.clang_arg(format!("-isystem{}", inc.display()));
                }
            }
        }
    }
    // Also include CRT / MinGW headers if present
    for sys_inc in &[
        r"C:\msys64\ucrt64\include",
        r"C:\msys64\mingw64\include",
    ] {
        let p = Path::new(sys_inc);
        if p.exists() {
            builder = builder.clang_arg(format!("-isystem{}", p.display()));
        }
    }

    let bindings = builder.generate();

    match bindings {
        Ok(b) => {
            let path = out_dir.join("pr_sdk_bindings.rs");
            b.write_to_file(&path)
                .expect("Couldn't write bindings!");
        }
        Err(e) => {
            panic!("Cannot generate bindings for Premiere Pro SDK: {:?}", e);
        }
    }

    // 3. Setup VCPKG and Compile FFmpeg statically
    let vcpkg_dir = if let Ok(root) = env::var("VCPKG_ROOT") {
        PathBuf::from(root)
    } else {
        out_dir.join("vcpkg")
    };
    
    let triplet = if target_os == "windows" {
        "x64-windows-static"
    } else if target_os == "macos" {
        if target_arch == "aarch64" {
            "arm64-osx"
        } else {
            "x64-osx"
        }
    } else {
        "x64-osx"
    };

    let ffmpeg_features = if target_os == "windows" {
        "ffmpeg[core,avcodec,avformat,swscale,swresample,vpx,dav1d,aom,nvcodec,amf,qsv]"
    } else {
        "ffmpeg[core,avcodec,avformat,swscale,swresample,vpx,dav1d,aom]"
    };

    let skip_ffmpeg_build = env::var("SKIP_FFMPEG_BUILD").is_ok();

    if skip_ffmpeg_build {
        println!("cargo:warning=SKIP_FFMPEG_BUILD is set. Skipping vcpkg compilation.");
    } else if !vcpkg_dir.exists() {
        println!("cargo:warning=Cloning vcpkg and compiling FFmpeg from source. This will take 15-40 minutes!");
        
        let status = Command::new("git")
            .args(["clone", "https://github.com/microsoft/vcpkg.git", vcpkg_dir.to_str().unwrap()])
            .status().unwrap();
        if !status.success() { panic!("Failed to clone vcpkg"); }

        let bootstrap_status = if target_os == "windows" {
            Command::new("cmd")
                .args(["/C", "bootstrap-vcpkg.bat"])
                .current_dir(&vcpkg_dir)
                .status().unwrap()
        } else {
            Command::new("sh")
                .arg("bootstrap-vcpkg.sh")
                .current_dir(&vcpkg_dir)
                .status().unwrap()
        };
        if !bootstrap_status.success() { panic!("Failed to bootstrap vcpkg"); }

        hack_portfile(&vcpkg_dir);

        let vcpkg_binary = if target_os == "windows" {
            vcpkg_dir.join("vcpkg.exe")
        } else {
            vcpkg_dir.join("vcpkg")
        };

        let status = run_command_streaming(
            &vcpkg_binary,
            &[
                "install",
                &format!("{}:{}", ffmpeg_features, triplet),
            ],
            &vcpkg_dir
        );
            
        if !status.success() {
            panic!("Failed to install ffmpeg via vcpkg");
        }
    } else if env::var("SKIP_FFMPEG_UPDATE").is_err() {
        // --- AUTOMATIC FFMPEG UPDATE ---
        println!("cargo:warning=Checking for FFmpeg updates in vcpkg...");
        
        let _ = Command::new("git")
            .args(["checkout", "--", "ports/ffmpeg/portfile.cmake"])
            .current_dir(&vcpkg_dir)
            .status();

        let pull_status = Command::new("git")
            .args(["pull"])
            .current_dir(&vcpkg_dir)
            .status().unwrap();

        if pull_status.success() {
            hack_portfile(&vcpkg_dir);

            let vcpkg_binary = if target_os == "windows" {
                vcpkg_dir.join("vcpkg.exe")
            } else {
                vcpkg_dir.join("vcpkg")
            };

            let _ = run_command_streaming(
                &vcpkg_binary,
                &[
                    "install",
                    &format!("{}:{}", ffmpeg_features, triplet),
                ],
                &vcpkg_dir
            );
        }
    }

    if !skip_ffmpeg_build {
        unsafe {
            if env::var("VCPKG_ROOT").is_err() {
                env::set_var("VCPKG_ROOT", &vcpkg_dir);
            }
        }
        vcpkg::Config::new()
            .target_triplet(triplet)
            .find_package("ffmpeg")
            .expect("Failed to link ffmpeg from vcpkg");
    }

    // Link target-specific libraries
    if target_os == "windows" {
        println!("cargo:rustc-link-lib=Secur32");
        println!("cargo:rustc-link-lib=Ncrypt");
        println!("cargo:rustc-link-lib=Crypt32");
        println!("cargo:rustc-link-lib=Bcrypt");
        println!("cargo:rustc-link-lib=Ole32");
        println!("cargo:rustc-link-lib=User32");
        println!("cargo:rustc-link-lib=Advapi32");
    } else if target_os == "macos" {
        println!("cargo:rustc-link-lib=framework=CoreFoundation");
        println!("cargo:rustc-link-lib=framework=CoreVideo");
        println!("cargo:rustc-link-lib=framework=CoreMedia");
        println!("cargo:rustc-link-lib=framework=VideoToolbox");
        println!("cargo:rustc-link-lib=framework=AudioToolbox");
        println!("cargo:rustc-link-lib=framework=Security");
        println!("cargo:rustc-link-lib=framework=AppKit");
        println!("cargo:rustc-link-lib=iconv");
        println!("cargo:rustc-link-lib=bz2");
        println!("cargo:rustc-link-lib=z");
    }


}
