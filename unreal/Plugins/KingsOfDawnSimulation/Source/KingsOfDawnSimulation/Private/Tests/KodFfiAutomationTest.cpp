// Unreal automation tests of the FFI roundtrip (run: Automation RunTests KingsOfDawn.Simulation).
#include "KodSimulationHandle.h"
#include "KodSimulationWorker.h"
#include "Misc/AutomationTest.h"

#if WITH_DEV_AUTOMATION_TESTS

IMPLEMENT_SIMPLE_AUTOMATION_TEST(FKodFfiRoundtripTest, "KingsOfDawn.Simulation.FfiRoundtrip",
	EAutomationTestFlags::EditorContext | EAutomationTestFlags::ClientContext | EAutomationTestFlags::EngineFilter)

bool FKodFfiRoundtripTest::RunTest(const FString& Parameters)
{
	TestTrue(TEXT("ABI compatible"), FKodSimulationHandle::IsAbiCompatible());

	FKodSimulationHandle Sim;
	TestEqual(TEXT("create"), Sim.Create(42, 1836, 1, 1), EKodStatus::Ok);
	TestEqual(TEXT("open"), Sim.OpenAccount(1, 1, 0, TEXT("Staatskasse"), 100000000000LL), EKodStatus::Ok);
	TestEqual(TEXT("open"), Sim.OpenAccount(2, 1, 0, TEXT("Händler"), 5000000000LL), EKodStatus::Ok);
	TestEqual(TEXT("transfer"), Sim.Transfer(2, 2, 1, 1, 2, 1), EKodStatus::Ok); // rejected at execution: not owner
	FKodStepResult Result;
	TestEqual(TEXT("step"), Sim.Step(2, Result), EKodStatus::Ok);
	TestEqual(TEXT("applied"), Result.Applied, 2u);
	TestEqual(TEXT("rejected"), Result.Rejected, 1u);
	int64 Balance = 0;
	TestEqual(TEXT("unknown account"), Sim.Balance(99, Balance), EKodStatus::NotFound);

	TArray<uint8> Bytes;
	TestEqual(TEXT("save"), Sim.Save(Bytes), EKodStatus::Ok);
	FKodSimulationHandle Loaded;
	TestEqual(TEXT("load"), Loaded.Load(Bytes), EKodStatus::Ok);
	FString A, B;
	Sim.StateHash(A);
	Loaded.StateHash(B);
	TestEqual(TEXT("hash after load"), A, B);

	Bytes[Bytes.Num() / 2] ^= 0x40;
	FKodSimulationHandle Corrupt;
	TestEqual(TEXT("corrupt save rejected"), Corrupt.Load(Bytes), EKodStatus::InvalidSave);
	TestFalse(TEXT("no handle on failure"), Corrupt.IsValid());
	return true;
}

IMPLEMENT_SIMPLE_AUTOMATION_TEST(FKodWorkerLifecycleTest, "KingsOfDawn.Simulation.WorkerLifecycle",
	EAutomationTestFlags::EditorContext | EAutomationTestFlags::ClientContext | EAutomationTestFlags::EngineFilter)

bool FKodWorkerLifecycleTest::RunTest(const FString& Parameters)
{
	FString Hash;
	{
		FKodSimulationWorker Worker;
		Worker.Enqueue([](FKodSimulationHandle& H) { H.Create(7, 1836, 1, 1); });
		Worker.Enqueue([](FKodSimulationHandle& H) { FKodStepResult R; H.Step(365, R); });
		Worker.Enqueue([&Hash](FKodSimulationHandle& H) { H.StateHash(Hash); });
		Worker.Flush();
		Worker.Shutdown();
		TestFalse(TEXT("no jobs after shutdown"), Worker.Enqueue([](FKodSimulationHandle&) {}));
	}
	FKodSimulationHandle Reference;
	Reference.Create(7, 1836, 1, 1);
	FKodStepResult R;
	Reference.Step(365, R);
	FString Expected;
	Reference.StateHash(Expected);
	TestEqual(TEXT("worker result equals direct result"), Hash, Expected);
	return true;
}

#endif
