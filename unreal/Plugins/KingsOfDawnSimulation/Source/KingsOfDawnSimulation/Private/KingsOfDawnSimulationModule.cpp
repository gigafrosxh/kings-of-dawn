#include "KingsOfDawnSimulationLog.h"
#include "KodSimulationHandle.h"
#include "KodSimulationWorker.h"
#include "Misc/CommandLine.h"
#include "Misc/CoreDelegates.h"
#include "Misc/Parse.h"
#include "Modules/ModuleManager.h"

DEFINE_LOG_CATEGORY(LogKodSimulation);

namespace
{
	/**
	 * Packaged-build smoke test (`-KodFfiSmoke`): FFI roundtrip on the worker
	 * thread (create, commands, step, save, load, hash compare), clean worker
	 * shutdown, then exit with status 0 (pass) or 1 (fail).
	 */
	void RunPackagedSmoke()
	{
		bool bPass = true;
		FString HashLive, HashLoaded;
		{
			FKodSimulationWorker Worker;
			Worker.Enqueue([&bPass, &HashLive, &HashLoaded](FKodSimulationHandle& Sim)
			{
				bPass &= Sim.Create(42, 1836, 1, 1) == EKodStatus::Ok;
				bPass &= Sim.OpenAccount(1, 1, 0, TEXT("Staatskasse"), 100000000000LL) == EKodStatus::Ok;
				FKodStepResult Result;
				bPass &= Sim.Step(365, Result) == EKodStatus::Ok && Result.Applied == 1;
				TArray<uint8> Bytes;
				bPass &= Sim.Save(Bytes) == EKodStatus::Ok;
				FKodSimulationHandle Loaded;
				bPass &= Loaded.Load(Bytes) == EKodStatus::Ok;
				Sim.StateHash(HashLive);
				Loaded.StateHash(HashLoaded);
				bPass &= !HashLive.IsEmpty() && HashLive == HashLoaded;
			});
			Worker.Flush();
			Worker.Shutdown();
		}
		UE_LOG(LogKodSimulation, Display, TEXT("KodFfiSmoke: %s hash=%s loaded=%s"), bPass ? TEXT("PASS") : TEXT("FAIL"), *HashLive, *HashLoaded);
		FPlatformMisc::RequestExitWithStatus(false, bPass ? 0 : 1);
	}
}

class FKingsOfDawnSimulationModule final : public IModuleInterface
{
public:
	virtual void StartupModule() override
	{
		if (!FKodSimulationHandle::IsAbiCompatible())
		{
			UE_LOG(LogKodSimulation, Fatal, TEXT("kod_ffi ABI mismatch: rebuild the Rust library (scripts/build-ffi)."));
		}
		UE_LOG(LogKodSimulation, Log, TEXT("kod_ffi ABI compatible"));
		if (FParse::Param(FCommandLine::Get(), TEXT("KodFfiSmoke")))
		{
			FCoreDelegates::OnPostEngineInit.AddStatic(&RunPackagedSmoke);
		}
	}
};

IMPLEMENT_MODULE(FKingsOfDawnSimulationModule, KingsOfDawnSimulation)
