// RAII owner of a kod_ffi simulation handle.
#pragma once

#include "CoreMinimal.h"

struct KodSim;

/** Status codes mirrored from kod_ffi.h (KodStatus), stable across ABI v1. */
enum class EKodStatus : int32
{
	Ok = 0,
	NullPointer = 1,
	InvalidArgument = 2,
	BufferTooSmall = 3,
	AbiMismatch = 4,
	SubmitRejected = 5,
	InvalidSave = 6,
	NotFound = 7,
	Overflow = 8,
	Panic = 9,
	Poisoned = 10,
};

/** Result of stepping the simulation. */
struct KINGSOFDAWNSIMULATION_API FKodStepResult
{
	uint64 NextTick = 0;
	uint32 Applied = 0;
	uint32 Rejected = 0;
	uint32 Events = 0;
};

/**
 * Move-only owner of one Rust simulation handle.
 * Not thread-safe: use from a single thread at a time (the worker).
 * Never exposes the raw pointer to UObjects or Blueprints.
 */
class KINGSOFDAWNSIMULATION_API FKodSimulationHandle
{
public:
	FKodSimulationHandle() = default;
	~FKodSimulationHandle();
	FKodSimulationHandle(const FKodSimulationHandle&) = delete;
	FKodSimulationHandle& operator=(const FKodSimulationHandle&) = delete;
	FKodSimulationHandle(FKodSimulationHandle&& Other) noexcept;
	FKodSimulationHandle& operator=(FKodSimulationHandle&& Other) noexcept;

	/** Verifies that the linked library implements the header's ABI version. */
	static bool IsAbiCompatible();
	/** Thread-local last error message of the library. */
	static FString LastError();

	EKodStatus Create(uint64 Seed, int32 Year, uint8 Month, uint8 Day);
	EKodStatus Load(const TArray<uint8>& SaveBytes);
	void Reset();
	bool IsValid() const { return Sim != nullptr; }

	EKodStatus Step(uint32 Ticks, FKodStepResult& OutResult);
	EKodStatus StateHash(FString& OutHex) const;
	EKodStatus Save(TArray<uint8>& OutBytes) const;
	EKodStatus NextTick(uint64& OutTick) const;

	// Fixture-domain commands (kod_core::fixture::Ledger) until real domains exist.
	EKodStatus OpenAccount(uint32 Actor, uint64 Sequence, uint64 ExecuteAt, const FString& Name, int64 DepositNanos);
	EKodStatus Transfer(uint32 Actor, uint64 Sequence, uint64 ExecuteAt, uint32 From, uint32 To, int64 AmountNanos);
	EKodStatus Balance(uint32 Account, int64& OutNanos) const;

private:
	KodSim* Sim = nullptr;
};
