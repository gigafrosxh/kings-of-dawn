#include "KodSimulationWorker.h"

#include "HAL/Event.h"
#include "HAL/PlatformProcess.h"
#include "HAL/RunnableThread.h"
#include "KingsOfDawnSimulationLog.h"

FKodSimulationWorker::FKodSimulationWorker()
{
	WakeUp = FPlatformProcess::GetSynchEventFromPool(false);
	Thread = FRunnableThread::Create(this, TEXT("KodSimulationWorker"), 0, TPri_Normal);
	check(Thread != nullptr);
}

FKodSimulationWorker::~FKodSimulationWorker()
{
	Shutdown();
}

bool FKodSimulationWorker::Enqueue(FJob Job)
{
	if (bStopping.load())
	{
		return false;
	}
	Jobs.Enqueue(MoveTemp(Job));
	WakeUp->Trigger();
	return true;
}

void FKodSimulationWorker::Flush()
{
	FEvent* Done = FPlatformProcess::GetSynchEventFromPool(true);
	if (Enqueue([Done](FKodSimulationHandle&) { Done->Trigger(); }))
	{
		Done->Wait();
	}
	FPlatformProcess::ReturnSynchEventToPool(Done);
}

uint32 FKodSimulationWorker::Run()
{
	for (;;)
	{
		FJob Job;
		while (Jobs.Dequeue(Job))
		{
			Job(Handle);
		}
		if (bStopping.load())
		{
			// Drain jobs that raced with Stop(), then release the handle on this thread.
			while (Jobs.Dequeue(Job))
			{
				Job(Handle);
			}
			Handle.Reset();
			return 0;
		}
		WakeUp->Wait();
	}
}

void FKodSimulationWorker::Stop()
{
	bStopping.store(true);
	if (WakeUp != nullptr)
	{
		WakeUp->Trigger();
	}
}

void FKodSimulationWorker::Shutdown()
{
	if (Thread == nullptr)
	{
		return;
	}
	Stop();
	Thread->WaitForCompletion();
	delete Thread;
	Thread = nullptr;
	FPlatformProcess::ReturnSynchEventToPool(WakeUp);
	WakeUp = nullptr;
	UE_LOG(LogKodSimulation, Log, TEXT("Simulation worker stopped"));
}
