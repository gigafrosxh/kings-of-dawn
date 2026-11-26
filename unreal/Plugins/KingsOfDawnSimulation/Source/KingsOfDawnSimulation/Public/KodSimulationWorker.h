// Dedicated simulation thread that exclusively owns the FFI handle.
#pragma once

#include "CoreMinimal.h"
#include "Containers/Queue.h"
#include "HAL/Runnable.h"
#include <atomic>
#include "KodSimulationHandle.h"

class FRunnableThread;
class FEvent;

/**
 * Executes jobs against the simulation handle on its own thread, in
 * submission order. Jobs must not capture UObjects by raw pointer; results
 * go back to the game thread via AsyncTask with weak pointers.
 */
class KINGSOFDAWNSIMULATION_API FKodSimulationWorker final : public FRunnable
{
public:
	using FJob = TFunction<void(FKodSimulationHandle&)>;

	FKodSimulationWorker();
	virtual ~FKodSimulationWorker() override;

	/** Thread-safe; returns false after shutdown started. */
	bool Enqueue(FJob Job);
	/** Blocks the caller until every job enqueued before the call has run. */
	void Flush();
	/** Stops accepting jobs, drains the queue, joins and releases the handle. */
	void Shutdown();

	// FRunnable
	virtual uint32 Run() override;
	virtual void Stop() override;

private:
	TQueue<FJob, EQueueMode::Mpsc> Jobs;
	FEvent* WakeUp = nullptr;
	FRunnableThread* Thread = nullptr;
	std::atomic<bool> bStopping{false};
	FKodSimulationHandle Handle; // touched only by the worker thread (and after join)
};
