# Builds the kod_ffi static library for Unreal on Windows at
# rust\target\x86_64-pc-windows-msvc\release\kod_ffi.lib (expected by KodFfi.Build.cs).
$ErrorActionPreference = "Stop"
Set-Location (Join-Path $PSScriptRoot "..")
$triple = "x86_64-pc-windows-msvc"
rustup target add $triple | Out-Null
cargo build --manifest-path rust/Cargo.toml -p kod-ffi --release --locked --target $triple
if ($LASTEXITCODE -ne 0) { throw "cargo build failed" }
Write-Host "built rust\target\$triple\release\kod_ffi.lib"
