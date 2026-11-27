// F-01 "Szenario wählen" panel (Slate) following mockups/f01.jpg:
// dark steel panels, brass accents, year presets, polity list, details with
// source quality, bookmarks, sandbox polities, start button.
#pragma once

#include "CoreMinimal.h"
#include "Widgets/SCompoundWidget.h"

class FKodScenarioViewModel;
class SVerticalBox;
class SEditableTextBox;

DECLARE_DELEGATE(FOnKodScenarioStarted);

class KINGSOFDAWNSIMULATION_API SKodScenarioPanel : public SCompoundWidget
{
public:
	enum class ETab : uint8 { Bookmarks, Historical, Alternate, Sandbox };

	SLATE_BEGIN_ARGS(SKodScenarioPanel) {}
		SLATE_ARGUMENT(TSharedPtr<FKodScenarioViewModel>, ViewModel)
		SLATE_EVENT(FOnKodScenarioStarted, OnStarted)
	SLATE_END_ARGS()

	void Construct(const FArguments& InArgs);
	virtual ~SKodScenarioPanel() override;

	// Interaction entry points (also used by the UI smoke test).
	FReply OnTab(ETab Tab);
	FReply OnPreset(int32 Year);
	FReply OnStep(int32 Delta);
	FReply OnPolity(uint32 PolityId);
	FReply OnAddBookmark();
	FReply OnApplyBookmark(uint32 BookmarkId);
	FReply OnRemoveBookmark(uint32 BookmarkId);
	FReply OnCreateCustomPolity();
	FReply OnStart();

	ETab GetActiveTab() const { return ActiveTab; }
	/** The panel accepts keyboard focus so UI-only input mode can focus it (#62). */
	virtual bool SupportsKeyboardFocus() const override { return true; }
	/** Visible texts of the details section, for tests: Startjahr, Spielmodus, Land, Quellengüte, Datenstatus. */
	TArray<FText> GetDetailTexts() const;

private:
	virtual void Tick(const FGeometry& AllottedGeometry, const double InCurrentTime, const float InDeltaTime) override;
	void MarkDirty() { bDirty = true; }
	void Rebuild();
	TSharedRef<SWidget> BuildSection(const FText& Title, const TSharedRef<SWidget>& Body) const;
	TSharedRef<SWidget> BuildTabs();
	TSharedRef<SWidget> BuildYearGrid();
	TSharedRef<SWidget> BuildPolities();
	TSharedRef<SWidget> BuildDetails();
	TSharedRef<SWidget> BuildBookmarks();
	TSharedRef<SWidget> BuildSandbox();
	TSharedRef<SWidget> MakeButton(const FText& Label, FOnClicked OnClicked, bool bActive, bool bEnabled = true) const;
	FText SelectedPolityName() const;
	FText DataStatusText() const;

	TSharedPtr<FKodScenarioViewModel> ViewModel;
	FOnKodScenarioStarted OnStarted;
	TSharedPtr<SVerticalBox> Content;
	TSharedPtr<SEditableTextBox> NameBox;
	ETab ActiveTab = ETab::Historical;
	FDelegateHandle ChangedHandle;
	bool bDirty = false;
};
