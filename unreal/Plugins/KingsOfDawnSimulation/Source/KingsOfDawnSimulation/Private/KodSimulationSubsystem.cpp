#include "KodSimulationSubsystem.h"

#include "Async/Async.h"
#include "KingsOfDawnSimulationLog.h"
#include "KodSimulationWorker.h"
#include "Misc/FileHelper.h"

void UKodSimulationSubsystem::Initialize(FSubsystemCollectionBase& Collection)
{
	Super::Initialize(Collection);
	Worker = MakeUnique<FKodSimulationWorker>();
}

void UKodSimulationSubsystem::Deinitialize()
{
	if (Worker)
	{
		Worker->Shutdown();
		Worker.Reset();
	}
	Super::Deinitialize();
}

void UKodSimulationSubsystem::StartSession(int64 Seed, int32 Year)
{
	if (!Worker)
	{
		return;
	}
	Worker->Enqueue([Seed, Year](FKodSimulationHandle& Handle)
	{
		const EKodStatus Status = Handle.Create(static_cast<uint64>(Seed), Year, 1, 1);
		if (Status != EKodStatus::Ok)
		{
			UE_LOG(LogKodSimulation, Error, TEXT("StartSession failed (%d): %s"), static_cast<int32>(Status), *FKodSimulationHandle::LastError());
		}
	});
}

void UKodSimulationSubsystem::AdvanceTicks(int32 Ticks)
{
	if (!Worker || Ticks <= 0)
	{
		return;
	}
	TWeakObjectPtr<UKodSimulationSubsystem> WeakThis(this);
	Worker->Enqueue([Ticks, WeakThis](FKodSimulationHandle& Handle)
	{
		FKodStepResult Result;
		const EKodStatus Status = Handle.Step(static_cast<uint32>(Ticks), Result);
		if (Status != EKodStatus::Ok)
		{
			UE_LOG(LogKodSimulation, Error, TEXT("AdvanceTicks failed (%d): %s"), static_cast<int32>(Status), *FKodSimulationHandle::LastError());
			return;
		}
		FString Hash;
		Handle.StateHash(Hash);
		AsyncTask(ENamedThreads::GameThread, [WeakThis, NextTick = Result.NextTick, Hash]()
		{
			if (UKodSimulationSubsystem* Self = WeakThis.Get())
			{
				Self->OnTicksAdvanced.Broadcast(static_cast<int64>(NextTick), Hash);
			}
		});
	});
}

void UKodSimulationSubsystem::SaveToFile(const FString& Path)
{
	if (!Worker)
	{
		return;
	}
	Worker->Enqueue([Path](FKodSimulationHandle& Handle)
	{
		TArray<uint8> Bytes;
		const EKodStatus Status = Handle.Save(Bytes);
		if (Status != EKodStatus::Ok || !FFileHelper::SaveArrayToFile(Bytes, *Path))
		{
			UE_LOG(LogKodSimulation, Error, TEXT("SaveToFile %s failed (%d): %s"), *Path, static_cast<int32>(Status), *FKodSimulationHandle::LastError());
		}
	});
}

void UKodSimulationSubsystem::LoadFromFile(const FString& Path)
{
	if (!Worker)
	{
		return;
	}
	Worker->Enqueue([Path](FKodSimulationHandle& Handle)
	{
		TArray<uint8> Bytes;
		if (!FFileHelper::LoadFileToArray(Bytes, *Path))
		{
			UE_LOG(LogKodSimulation, Error, TEXT("LoadFromFile: cannot read %s"), *Path);
			return;
		}
		const EKodStatus Status = Handle.Load(Bytes);
		if (Status != EKodStatus::Ok)
		{
			UE_LOG(LogKodSimulation, Error, TEXT("LoadFromFile %s failed (%d): %s"), *Path, static_cast<int32>(Status), *FKodSimulationHandle::LastError());
		}
	});
}

void UKodSimulationSubsystem::Flush()
{
	if (Worker)
	{
		Worker->Flush();
	}
}
