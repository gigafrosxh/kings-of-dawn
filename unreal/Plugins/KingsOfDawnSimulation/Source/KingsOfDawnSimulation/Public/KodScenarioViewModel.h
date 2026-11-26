// F-01 scenario setup view model: owns the Rust setup handle and exposes a
// parsed read model plus typed commands to the Slate panel.
#pragma once

#include "CoreMinimal.h"

struct KodScenario;

/** One polity offered for the selected year. */
struct KINGSOFDAWNSIMULATION_API FKodPolityItem
{
	uint32 Id = 0;
	FString Name;
	FString Quality; // synthetic | approximate | reconstructed | documented
};

/** One saved bookmark. */
struct KINGSOFDAWNSIMULATION_API FKodBookmarkItem
{
	uint32 Id = 0;
	FString Name;
	int32 Year = 0;
};

/** Parsed `kod_scenario_view_json` snapshot (schema 1). */
struct KINGSOFDAWNSIMULATION_API FKodScenarioView
{
	int32 Year = 0;
	FString YearLabel;
	int32 MinYear = 0;
	int32 MaxYear = 0;
	TArray<TPair<int32, FString>> Presets;
	FString Mode; // historical | alternate | sandbox
	uint32 SelectedPolity = 0;
	FString DatasetId;
	FString DatasetVersion;
	FString StatusKind; // complete | partial | missing
	FString StatusReason;
	FString MinQuality;
	TArray<FKodPolityItem> Polities;
	TArray<FKodBookmarkItem> Bookmarks;
	bool bConfirmed = false;
	bool bCustomPolitiesAllowed = false;

	static bool Parse(const FString& Json, FKodScenarioView& Out);
};

/**
 * Game-thread view model. Setup commands are tiny and run synchronously on
 * the calling thread; the running simulation uses FKodSimulationWorker.
 */
class KINGSOFDAWNSIMULATION_API FKodScenarioViewModel : public TSharedFromThis<FKodScenarioViewModel>
{
public:
	DECLARE_MULTICAST_DELEGATE(FOnChanged);

	explicit FKodScenarioViewModel(int32 CurrentYear);
	~FKodScenarioViewModel();
	FKodScenarioViewModel(const FKodScenarioViewModel&) = delete;
	FKodScenarioViewModel& operator=(const FKodScenarioViewModel&) = delete;

	bool IsValid() const { return Handle != nullptr; }
	const FKodScenarioView& GetView() const { return View; }
	/** Last rejection/error message (empty after a successful command). */
	const FText& GetLastError() const { return LastError; }
	FOnChanged& OnChanged() { return Changed; }

	bool SelectYear(int32 Year);
	bool StepYear(int32 Delta);
	bool SelectMode(uint8 Mode); // 0 historical, 1 alternate, 2 sandbox
	bool SelectPolity(uint32 PolityId);
	bool CreateCustomPolity(const FString& Name);
	bool AddBookmark(const FString& Name);
	bool ApplyBookmark(uint32 BookmarkId);
	bool RemoveBookmark(uint32 BookmarkId);
	bool Confirm();

	bool Save(TArray<uint8>& OutBytes) const;
	bool Load(const TArray<uint8>& Bytes);

	/** German label for a quality key ("Synthetisch (Testdaten)", ...). */
	static FText QualityText(const FString& Quality);
	/** German label for a mode key. */
	static FText ModeText(const FString& Mode);

private:
	bool Finish(int32 Status, uint32 Code);
	void Refresh();

	KodScenario* Handle = nullptr;
	FKodScenarioView View;
	FText LastError;
	FOnChanged Changed;
};
