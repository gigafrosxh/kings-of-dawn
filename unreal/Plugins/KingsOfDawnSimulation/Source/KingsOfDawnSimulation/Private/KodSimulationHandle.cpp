#include "KodSimulationHandle.h"

#include "kod_ffi.h"

static_assert(static_cast<int32>(EKodStatus::Poisoned) == KOD_STATUS_POISONED, "EKodStatus out of sync with kod_ffi.h");
static_assert(static_cast<int32>(EKodStatus::BufferTooSmall) == KOD_STATUS_BUFFER_TOO_SMALL, "EKodStatus out of sync with kod_ffi.h");

namespace
{
	EKodStatus ToStatus(KodStatus Status) { return static_cast<EKodStatus>(static_cast<int32>(Status)); }
}

FKodSimulationHandle::~FKodSimulationHandle()
{
	Reset();
}

FKodSimulationHandle::FKodSimulationHandle(FKodSimulationHandle&& Other) noexcept
	: Sim(Other.Sim)
{
	Other.Sim = nullptr;
}

FKodSimulationHandle& FKodSimulationHandle::operator=(FKodSimulationHandle&& Other) noexcept
{
	if (this != &Other)
	{
		Reset();
		Sim = Other.Sim;
		Other.Sim = nullptr;
	}
	return *this;
}

bool FKodSimulationHandle::IsAbiCompatible()
{
	return kod_abi_version() == KOD_ABI_VERSION;
}

FString FKodSimulationHandle::LastError()
{
	size_t Len = 0;
	kod_last_error(nullptr, 0, &Len);
	TArray<uint8> Buffer;
	Buffer.SetNumZeroed(static_cast<int32>(Len) + 1);
	if (kod_last_error(Buffer.GetData(), Len, &Len) != KOD_STATUS_OK)
	{
		return FString();
	}
	return FString(UTF8_TO_TCHAR(reinterpret_cast<const ANSICHAR*>(Buffer.GetData())));
}

void FKodSimulationHandle::Reset()
{
	if (Sim != nullptr)
	{
		kod_sim_destroy(Sim);
		Sim = nullptr;
	}
}

EKodStatus FKodSimulationHandle::Create(uint64 Seed, int32 Year, uint8 Month, uint8 Day)
{
	Reset();
	const KodDate Start{Year, Month, Day};
	return ToStatus(kod_sim_create(KOD_ABI_VERSION, Seed, Start, &Sim));
}

EKodStatus FKodSimulationHandle::Load(const TArray<uint8>& SaveBytes)
{
	KodSim* Loaded = nullptr;
	const KodStatus Status = kod_sim_load(KOD_ABI_VERSION, SaveBytes.GetData(), static_cast<size_t>(SaveBytes.Num()), &Loaded);
	if (Status == KOD_STATUS_OK)
	{
		Reset();
		Sim = Loaded;
	}
	return ToStatus(Status);
}

EKodStatus FKodSimulationHandle::Step(uint32 Ticks, FKodStepResult& OutResult)
{
	KodStepSummary Summary{};
	const KodStatus Status = kod_sim_step(Sim, Ticks, &Summary);
	OutResult = FKodStepResult{Summary.next_tick, Summary.applied, Summary.rejected, Summary.events};
	return ToStatus(Status);
}

EKodStatus FKodSimulationHandle::StateHash(FString& OutHex) const
{
	KodHash128 Hash{};
	const KodStatus Status = kod_sim_state_hash(Sim, &Hash);
	OutHex = FString::Printf(TEXT("%016llx%016llx"), static_cast<unsigned long long>(Hash.hi), static_cast<unsigned long long>(Hash.lo));
	return ToStatus(Status);
}

EKodStatus FKodSimulationHandle::Save(TArray<uint8>& OutBytes) const
{
	size_t Len = 0;
	KodStatus Status = kod_sim_save(Sim, nullptr, 0, &Len);
	if (Status != KOD_STATUS_BUFFER_TOO_SMALL)
	{
		return ToStatus(Status);
	}
	OutBytes.SetNumUninitialized(static_cast<int32>(Len));
	Status = kod_sim_save(Sim, OutBytes.GetData(), Len, &Len);
	return ToStatus(Status);
}

EKodStatus FKodSimulationHandle::NextTick(uint64& OutTick) const
{
	uint64_t Tick = 0;
	const KodStatus Status = kod_sim_next_tick(Sim, &Tick);
	OutTick = Tick;
	return ToStatus(Status);
}

EKodStatus FKodSimulationHandle::OpenAccount(uint32 Actor, uint64 Sequence, uint64 ExecuteAt, const FString& Name, int64 DepositNanos)
{
	const FTCHARToUTF8 Utf8(*Name);
	return ToStatus(kod_ledger_open(Sim, Actor, Sequence, ExecuteAt, reinterpret_cast<const uint8_t*>(Utf8.Get()), static_cast<size_t>(Utf8.Length()), DepositNanos));
}

EKodStatus FKodSimulationHandle::Transfer(uint32 Actor, uint64 Sequence, uint64 ExecuteAt, uint32 From, uint32 To, int64 AmountNanos)
{
	return ToStatus(kod_ledger_transfer(Sim, Actor, Sequence, ExecuteAt, From, To, AmountNanos));
}

EKodStatus FKodSimulationHandle::Balance(uint32 Account, int64& OutNanos) const
{
	int64_t Value = 0;
	const KodStatus Status = kod_ledger_balance(Sim, Account, &Value);
	OutNanos = Value;
	return ToStatus(Status);
}
