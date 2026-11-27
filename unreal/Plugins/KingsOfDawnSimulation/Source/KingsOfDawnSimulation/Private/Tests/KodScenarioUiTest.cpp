// F-01 UI smoke test: builds the Slate panel against the real Rust setup and
// drives it through its interaction entry points.
#include "Framework/Application/SlateApplication.h"
#include "KodScenarioViewModel.h"
#include "Misc/AutomationTest.h"
#include "SKodScenarioPanel.h"

#if WITH_DEV_AUTOMATION_TESTS

IMPLEMENT_SIMPLE_AUTOMATION_TEST(FKodScenarioUiSmokeTest, "KingsOfDawn.Scenario.UiSmoke",
	EAutomationTestFlags::EditorContext | EAutomationTestFlags::ClientContext | EAutomationTestFlags::EngineFilter)

bool FKodScenarioUiSmokeTest::RunTest(const FString& Parameters)
{
	TSharedRef<FKodScenarioViewModel> VM = MakeShared<FKodScenarioViewModel>(2026);
	TestTrue(TEXT("setup handle"), VM->IsValid());
	bool bStarted = false;
	TSharedRef<SKodScenarioPanel> Panel = SNew(SKodScenarioPanel)
		.ViewModel(VM)
		.OnStarted(FOnKodScenarioStarted::CreateLambda([&bStarted]() { bStarted = true; }));

	TestTrue(TEXT("panel is keyboard focusable (#62)"), Panel->SupportsKeyboardFocus());
	TestEqual(TEXT("presets"), VM->GetView().Presets.Num(), 9);
	TestEqual(TEXT("starts at 100 BC"), VM->GetView().YearLabel, FString(TEXT("100 v. Chr.")));

	// 1 v. Chr. -> +1 -> 1 n. Chr.
	Panel->OnPreset(0);
	TestEqual(TEXT("1 BC"), VM->GetView().YearLabel, FString(TEXT("1 v. Chr.")));
	Panel->OnStep(1);
	TestEqual(TEXT("1 AD"), VM->GetView().YearLabel, FString(TEXT("1 n. Chr.")));

	// Missing data is visible in the details.
	Panel->OnPreset(650);
	TestTrue(TEXT("gap visible"), Panel->GetDetailTexts()[4].ToString().StartsWith(TEXT("Lückenhaft")));

	// Start requires a polity; selecting one and starting confirms.
	Panel->OnPreset(1936);
	Panel->OnStart();
	TestFalse(TEXT("not started without polity"), bStarted);
	TestFalse(TEXT("error shown"), VM->GetLastError().IsEmpty());
	Panel->OnPolity(VM->GetView().Polities[0].Id);
	TestEqual(TEXT("country shown"), Panel->GetDetailTexts()[2].ToString(), VM->GetView().Polities[0].Name);
	TestEqual(TEXT("quality shown"), Panel->GetDetailTexts()[3].ToString(), FString(TEXT("Synthetisch (Testdaten)")));

	// Sandbox tab switches mode.
	Panel->OnTab(SKodScenarioPanel::ETab::Sandbox);
	TestEqual(TEXT("sandbox"), VM->GetView().Mode, FString(TEXT("sandbox")));
	Panel->OnTab(SKodScenarioPanel::ETab::Historical);

	Panel->OnStart();
	TestTrue(TEXT("started"), bStarted);
	TestTrue(TEXT("confirmed"), VM->GetView().bConfirmed);

	// Source stays fixed across save/load.
	TArray<uint8> Bytes;
	TestTrue(TEXT("save"), VM->Save(Bytes));
	TSharedRef<FKodScenarioViewModel> Loaded = MakeShared<FKodScenarioViewModel>(2026);
	TestTrue(TEXT("load"), Loaded->Load(Bytes));
	TestEqual(TEXT("dataset fixed"), Loaded->GetView().DatasetId, FString(TEXT("kod.fixture.m0")));
	TestEqual(TEXT("year kept"), Loaded->GetView().Year, 1936);
	return true;
}

#endif
