#include "KodScenarioScreenSubsystem.h"

#include "Engine/GameViewportClient.h"
#include "Engine/LocalPlayer.h"
#include "Framework/Application/SlateApplication.h"
#include "GameFramework/PlayerController.h"
#include "KingsOfDawnSimulationLog.h"
#include "KodScenarioViewModel.h"
#include "Misc/CommandLine.h"
#include "Misc/FileHelper.h"
#include "Misc/Parse.h"
#include "Misc/Paths.h"
#include "TimerManager.h"
#include "UnrealClient.h"
#include "ImageUtils.h"
#include "Widgets/SWindow.h"
#include "Engine/World.h"
#include "SKodScenarioPanel.h"
#include "Widgets/Layout/SBox.h"

void UKodScenarioScreenSubsystem::PlayerControllerChanged(APlayerController* NewPlayerController)
{
	Super::PlayerControllerChanged(NewPlayerController);
	if (NewPlayerController == nullptr || Panel.IsValid() || FParse::Param(FCommandLine::Get(), TEXT("KodFfiSmoke")) || FParse::Param(FCommandLine::Get(), TEXT("KodNoScenarioScreen")))
	{
		return;
	}
	Show(NewPlayerController);
}

void UKodScenarioScreenSubsystem::Show(APlayerController* PC)
{
	UGameViewportClient* Viewport = GetLocalPlayer() ? GetLocalPlayer()->ViewportClient : nullptr;
	if (Viewport == nullptr)
	{
		return;
	}
	// The host supplies the current calendar year; the simulation never reads the clock.
	ViewModel = MakeShared<FKodScenarioViewModel>(FDateTime::Now().GetYear());
	TSharedRef<SKodScenarioPanel> PanelWidget = SNew(SKodScenarioPanel)
		.ViewModel(ViewModel)
		.OnStarted(FOnKodScenarioStarted::CreateUObject(this, &UKodScenarioScreenSubsystem::HandleStarted));
	Panel = SNew(SBox).HAlign(HAlign_Left).VAlign(VAlign_Fill).Padding(FMargin(12.f, 60.f, 0.f, 12.f))[PanelWidget];
	Viewport->AddViewportWidgetContent(Panel.ToSharedRef(), 10);

	Controller = PC;
	PC->SetShowMouseCursor(true);
	FInputModeUIOnly Mode;
	Mode.SetWidgetToFocus(PanelWidget);
	PC->SetInputMode(Mode);
	UE_LOG(LogKodSimulation, Log, TEXT("Scenario selection shown"));

	// Development aid for visual checks: -KodScenarioScreenshot=<path> captures the
	// viewport after the UI settled and quits.
	FString ShotPath;
	if (FParse::Value(FCommandLine::Get(), TEXT("KodScenarioScreenshot="), ShotPath) && PC->GetWorld())
	{
		FTimerHandle Shot, Quit;
		TWeakObjectPtr<UKodScenarioScreenSubsystem> WeakThis(this);
		PC->GetWorld()->GetTimerManager().SetTimer(Shot, [WeakThis, ShotPath]()
		{
			if (WeakThis.IsValid())
			{
				WeakThis->CaptureWindow(ShotPath);
			}
		}, 3.f, false);
		PC->GetWorld()->GetTimerManager().SetTimer(Quit, []() { FPlatformMisc::RequestExit(false); }, 6.f, false);
	}
}

void UKodScenarioScreenSubsystem::Hide()
{
	if (Panel.IsValid() && GetLocalPlayer() && GetLocalPlayer()->ViewportClient)
	{
		GetLocalPlayer()->ViewportClient->RemoveViewportWidgetContent(Panel.ToSharedRef());
	}
	Panel.Reset();
	if (APlayerController* PC = Controller.Get())
	{
		PC->SetInputMode(FInputModeGameAndUI());
	}
}

void UKodScenarioScreenSubsystem::HandleStarted()
{
	SaveSetup();
	UE_LOG(LogKodSimulation, Log, TEXT("Scenario confirmed: %s, polity %u, dataset %s %s"),
		*ViewModel->GetView().YearLabel, ViewModel->GetView().SelectedPolity, *ViewModel->GetView().DatasetId, *ViewModel->GetView().DatasetVersion);
	Hide();
}

bool UKodScenarioScreenSubsystem::CaptureWindow(const FString& Path) const
{
	// Slate capture of the whole game window (viewport + UI); works in editor -game
	// and packaged builds alike (#63), unlike FScreenshotRequest.
	UGameViewportClient* Viewport = GetLocalPlayer() ? GetLocalPlayer()->ViewportClient : nullptr;
	const TSharedPtr<SWindow> Window = Viewport ? Viewport->GetWindow() : nullptr;
	TArray<FColor> Pixels;
	FIntVector Size;
	if (!Window.IsValid() || !FSlateApplication::Get().TakeScreenshot(Window.ToSharedRef(), Pixels, Size) || Pixels.Num() == 0)
	{
		UE_LOG(LogKodSimulation, Error, TEXT("Scenario screenshot failed: no window capture"));
		return false;
	}
	for (FColor& Pixel : Pixels)
	{
		Pixel.A = 255;
	}
	const FImageView Image(Pixels.GetData(), Size.X, Size.Y);
	TArray64<uint8> Png;
	if (!FImageUtils::CompressImage(Png, TEXT("png"), Image))
	{
		UE_LOG(LogKodSimulation, Error, TEXT("Scenario screenshot failed: PNG encoding"));
		return false;
	}
	// Packaged macOS builds run in the App Sandbox and may not write outside their
	// container; fall back to the game's screenshot directory and log the real path.
	FString Target = FPaths::IsRelative(Path) ? FPaths::Combine(FPaths::ScreenShotDir(), Path) : Path;
	bool bSaved = FFileHelper::SaveArrayToFile(Png, *Target);
	if (!bSaved)
	{
		Target = FPaths::Combine(FPaths::ScreenShotDir(), FPaths::GetCleanFilename(Path));
		bSaved = FFileHelper::SaveArrayToFile(Png, *Target);
	}
	Target = FPaths::ConvertRelativePathToFull(Target);
	UE_LOG(LogKodSimulation, Display, TEXT("Scenario screenshot %s: %s (%dx%d)"), bSaved ? TEXT("written") : TEXT("FAILED"), *Target, Size.X, Size.Y);
	return bSaved;
}

bool UKodScenarioScreenSubsystem::SaveSetup() const
{
	TArray<uint8> Bytes;
	if (!ViewModel.IsValid() || !ViewModel->Save(Bytes))
	{
		return false;
	}
	const FString Path = FPaths::Combine(FPaths::ProjectSavedDir(), TEXT("KodScenario.kodsave"));
	return FFileHelper::SaveArrayToFile(Bytes, *Path);
}

void UKodScenarioScreenSubsystem::Deinitialize()
{
	Hide();
	ViewModel.Reset();
	Super::Deinitialize();
}
