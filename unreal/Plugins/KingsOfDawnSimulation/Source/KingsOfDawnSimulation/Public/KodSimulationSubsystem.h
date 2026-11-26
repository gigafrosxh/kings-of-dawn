// Owns the simulation worker thread for the lifetime of the game instance.
#pragma once

#include "CoreMinimal.h"
#include "Subsystems/GameInstanceSubsystem.h"
#include "KodSimulationSubsystem.generated.h"

class FKodSimulationWorker;

DECLARE_DYNAMIC_MULTICAST_DELEGATE_TwoParams(FKodTicksAdvanced, int64, NextTick, const FString&, StateHash);

/**
 * Game-thread facade of the Rust simulation.
 *
 * All simulation work runs on one dedicated worker thread that exclusively
 * owns the FFI handle; the game thread only enqueues jobs and receives
 * results (delegates fire on the game thread). Deinitialize stops the worker,
 * joins it and releases the handle before the subsystem is destroyed.
 */
UCLASS()
class KINGSOFDAWNSIMULATION_API UKodSimulationSubsystem : public UGameInstanceSubsystem
{
	GENERATED_BODY()

public:
	virtual void Initialize(FSubsystemCollectionBase& Collection) override;
	virtual void Deinitialize() override;

	/** Starts a new session (replaces any running one). */
	UFUNCTION(BlueprintCallable, Category = "Kings of Dawn|Simulation")
	void StartSession(int64 Seed, int32 Year);

	/** Advances the simulation by Ticks days asynchronously; OnTicksAdvanced fires when done. */
	UFUNCTION(BlueprintCallable, Category = "Kings of Dawn|Simulation")
	void AdvanceTicks(int32 Ticks);

	/** Writes a save file asynchronously. */
	UFUNCTION(BlueprintCallable, Category = "Kings of Dawn|Simulation")
	void SaveToFile(const FString& Path);

	/** Loads a save file asynchronously (replaces the running session on success). */
	UFUNCTION(BlueprintCallable, Category = "Kings of Dawn|Simulation")
	void LoadFromFile(const FString& Path);

	UPROPERTY(BlueprintAssignable, Category = "Kings of Dawn|Simulation")
	FKodTicksAdvanced OnTicksAdvanced;

	/** Blocks until all queued jobs have run (tests and shutdown). */
	void Flush();

private:
	TUniquePtr<FKodSimulationWorker> Worker;
};
