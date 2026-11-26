using System;
using System.IO;
using UnrealBuildTool;

// External module wrapping the Rust static library `kod_ffi` (rust/crates/ffi).
// Build the library first: scripts/build-ffi.sh (macOS/Linux) or scripts/build-ffi.ps1 (Windows).
public class KodFfi : ModuleRules
{
	public KodFfi(ReadOnlyTargetRules Target) : base(Target)
	{
		Type = ModuleType.External;
		PublicSystemIncludePaths.Add(Path.Combine(ModuleDirectory, "include"));

		// <repo>/unreal/Plugins/KingsOfDawnSimulation/Source/ThirdParty/KodFfi -> <repo>/rust/target
		string RepoRoot = Path.GetFullPath(Path.Combine(ModuleDirectory, "..", "..", "..", "..", "..", ".."));
		string CargoTarget = Environment.GetEnvironmentVariable("KOD_CARGO_TARGET_DIR") ?? Path.Combine(RepoRoot, "rust", "target");

		string Triple;
		string LibName;
		if (Target.Platform == UnrealTargetPlatform.Win64)
		{
			Triple = "x86_64-pc-windows-msvc";
			LibName = "kod_ffi.lib";
			// System libraries required by the Rust standard library (cargo --print native-static-libs).
			PublicSystemLibraries.AddRange(new[] { "kernel32.lib", "advapi32.lib", "bcrypt.lib", "ntdll.lib", "userenv.lib", "ws2_32.lib" });
		}
		else if (Target.Platform == UnrealTargetPlatform.Mac)
		{
			Triple = Target.Architecture == UnrealArch.X64 ? "x86_64-apple-darwin" : "aarch64-apple-darwin";
			LibName = "libkod_ffi.a";
		}
		else if (Target.Platform == UnrealTargetPlatform.Linux)
		{
			Triple = "x86_64-unknown-linux-gnu";
			LibName = "libkod_ffi.a";
			PublicSystemLibraries.AddRange(new[] { "pthread", "dl", "m" });
		}
		else
		{
			throw new BuildException("KodFfi: unsupported platform " + Target.Platform);
		}

		string LibPath = Path.Combine(CargoTarget, Triple, "release", LibName);
		if (!File.Exists(LibPath))
		{
			throw new BuildException("KodFfi: Rust library not found at " + LibPath +
				". Run scripts/build-ffi.sh or scripts/build-ffi.ps1 first.");
		}
		PublicAdditionalLibraries.Add(LibPath);
	}
}
