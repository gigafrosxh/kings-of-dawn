// Shows the F-01 scenario selection when a local player gets a controller.
#pragma once

#include "CoreMinimal.h"
#include "Subsystems/LocalPlayerSubsystem.h"
#include "KodScenarioScreenSubsystem.generated.h"

class FKodScenarioViewModel;
class SWidget;

UCLASS()
class KINGSOFDAWNSIMULATION_API UKodScenarioScreenSubsystem : public ULocalPlayerSubsystem
{
	GENERATED_BODY()

public:
	virtual void PlayerControllerChanged(APlayerController* NewPlayerController) override;
	virtual void Deinitialize() override;

	/** Saves the confirmed (or current) setup to `Saved/KodScenario.kodsave`. */
	bool SaveSetup() const;

	/** Writes a PNG of the game window including UI (development/test aid). */
	bool CaptureWindow(const FString& Path) const;

private:
	void Show(APlayerController* PC);
	void Hide();
	void HandleStarted();

	TSharedPtr<FKodScenarioViewModel> ViewModel;
	TSharedPtr<SWidget> Panel;
	TWeakObjectPtr<APlayerController> Controller;
};
