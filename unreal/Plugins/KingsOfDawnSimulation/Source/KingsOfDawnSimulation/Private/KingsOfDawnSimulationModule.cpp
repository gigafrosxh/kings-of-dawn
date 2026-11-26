#include "KingsOfDawnSimulationLog.h"
#include "KodSimulationHandle.h"
#include "Modules/ModuleManager.h"

DEFINE_LOG_CATEGORY(LogKodSimulation);

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
	}
};

IMPLEMENT_MODULE(FKingsOfDawnSimulationModule, KingsOfDawnSimulation)
