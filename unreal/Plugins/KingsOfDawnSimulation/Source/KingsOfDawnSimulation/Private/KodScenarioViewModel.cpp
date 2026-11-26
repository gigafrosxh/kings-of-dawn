#include "KodScenarioViewModel.h"

#include "Dom/JsonObject.h"
#include "KingsOfDawnSimulationLog.h"
#include "KodSimulationHandle.h"
#include "Serialization/JsonReader.h"
#include "Serialization/JsonSerializer.h"
#include "kod_ffi.h"

#define LOCTEXT_NAMESPACE "KodScenario"

bool FKodScenarioView::Parse(const FString& Json, FKodScenarioView& Out)
{
	TSharedPtr<FJsonObject> Root;
	const TSharedRef<TJsonReader<>> Reader = TJsonReaderFactory<>::Create(Json);
	if (!FJsonSerializer::Deserialize(Reader, Root) || !Root.IsValid() || Root->GetIntegerField(TEXT("schema")) != 1)
	{
		return false;
	}
	FKodScenarioView V;
	const TSharedPtr<FJsonObject> Selected = Root->GetObjectField(TEXT("selected"));
	V.Year = Selected->GetIntegerField(TEXT("year"));
	V.YearLabel = Selected->GetStringField(TEXT("label"));
	const TArray<TSharedPtr<FJsonValue>>& Range = Root->GetArrayField(TEXT("range"));
	V.MinYear = static_cast<int32>(Range[0]->AsNumber());
	V.MaxYear = static_cast<int32>(Range[1]->AsNumber());
	for (const TSharedPtr<FJsonValue>& P : Root->GetArrayField(TEXT("presets")))
	{
		const TSharedPtr<FJsonObject> O = P->AsObject();
		V.Presets.Emplace(O->GetIntegerField(TEXT("year")), O->GetStringField(TEXT("label")));
	}
	V.Mode = Root->GetStringField(TEXT("mode"));
	V.SelectedPolity = static_cast<uint32>(Root->GetIntegerField(TEXT("polity")));
	const TSharedPtr<FJsonObject> Dataset = Root->GetObjectField(TEXT("dataset"));
	V.DatasetId = Dataset->GetStringField(TEXT("id"));
	V.DatasetVersion = Dataset->GetStringField(TEXT("version"));
	const TSharedPtr<FJsonObject> Status = Root->GetObjectField(TEXT("status"));
	V.StatusKind = Status->GetStringField(TEXT("kind"));
	Status->TryGetStringField(TEXT("reason"), V.StatusReason);
	Root->TryGetStringField(TEXT("min_quality"), V.MinQuality);
	for (const TSharedPtr<FJsonValue>& P : Root->GetArrayField(TEXT("polities")))
	{
		const TSharedPtr<FJsonObject> O = P->AsObject();
		V.Polities.Add({static_cast<uint32>(O->GetIntegerField(TEXT("id"))), O->GetStringField(TEXT("name")), O->GetStringField(TEXT("quality"))});
	}
	for (const TSharedPtr<FJsonValue>& B : Root->GetArrayField(TEXT("bookmarks")))
	{
		const TSharedPtr<FJsonObject> O = B->AsObject();
		V.Bookmarks.Add({static_cast<uint32>(O->GetIntegerField(TEXT("id"))), O->GetStringField(TEXT("name")), O->GetIntegerField(TEXT("year"))});
	}
	V.bConfirmed = Root->GetBoolField(TEXT("confirmed"));
	V.bCustomPolitiesAllowed = Root->GetBoolField(TEXT("custom_polities_allowed"));
	Out = MoveTemp(V);
	return true;
}

FKodScenarioViewModel::FKodScenarioViewModel(int32 CurrentYear)
{
	if (kod_scenario_create(KOD_ABI_VERSION, CurrentYear, &Handle) != KOD_STATUS_OK)
	{
		UE_LOG(LogKodSimulation, Error, TEXT("kod_scenario_create failed: %s"), *FKodSimulationHandle::LastError());
		Handle = nullptr;
		return;
	}
	Refresh();
}

FKodScenarioViewModel::~FKodScenarioViewModel()
{
	kod_scenario_destroy(Handle);
	Handle = nullptr;
}

void FKodScenarioViewModel::Refresh()
{
	size_t Len = 0;
	if (kod_scenario_view_json(Handle, nullptr, 0, &Len) != KOD_STATUS_BUFFER_TOO_SMALL)
	{
		return;
	}
	TArray<uint8> Buffer;
	Buffer.SetNumZeroed(static_cast<int32>(Len) + 1);
	if (kod_scenario_view_json(Handle, Buffer.GetData(), Len, &Len) != KOD_STATUS_OK)
	{
		return;
	}
	const FString Json = FString(UTF8_TO_TCHAR(reinterpret_cast<const ANSICHAR*>(Buffer.GetData())));
	if (!FKodScenarioView::Parse(Json, View))
	{
		UE_LOG(LogKodSimulation, Error, TEXT("Cannot parse scenario view"));
	}
	Changed.Broadcast();
}

bool FKodScenarioViewModel::Finish(int32 Status, uint32 Code)
{
	if (Status != KOD_STATUS_OK || Code != 0)
	{
		LastError = FText::FromString(FKodSimulationHandle::LastError());
		Changed.Broadcast();
		return false;
	}
	LastError = FText::GetEmpty();
	Refresh();
	return true;
}

bool FKodScenarioViewModel::SelectYear(int32 Year)
{
	uint32 Code = 0;
	return Finish(kod_scenario_select_year(Handle, Year, &Code), Code);
}

bool FKodScenarioViewModel::StepYear(int32 Delta)
{
	uint32 Code = 0;
	return Finish(kod_scenario_step_year(Handle, Delta, &Code), Code);
}

bool FKodScenarioViewModel::SelectMode(uint8 Mode)
{
	uint32 Code = 0;
	return Finish(kod_scenario_select_mode(Handle, Mode, &Code), Code);
}

bool FKodScenarioViewModel::SelectPolity(uint32 PolityId)
{
	uint32 Code = 0;
	return Finish(kod_scenario_select_polity(Handle, PolityId, &Code), Code);
}

bool FKodScenarioViewModel::CreateCustomPolity(const FString& Name)
{
	const FTCHARToUTF8 Utf8(*Name);
	uint32 Code = 0;
	return Finish(kod_scenario_create_custom_polity(Handle, reinterpret_cast<const uint8_t*>(Utf8.Get()), static_cast<size_t>(Utf8.Length()), &Code), Code);
}

bool FKodScenarioViewModel::AddBookmark(const FString& Name)
{
	const FTCHARToUTF8 Utf8(*Name);
	uint32 Code = 0;
	return Finish(kod_scenario_add_bookmark(Handle, reinterpret_cast<const uint8_t*>(Utf8.Get()), static_cast<size_t>(Utf8.Length()), &Code), Code);
}

bool FKodScenarioViewModel::ApplyBookmark(uint32 BookmarkId)
{
	uint32 Code = 0;
	return Finish(kod_scenario_apply_bookmark(Handle, BookmarkId, &Code), Code);
}

bool FKodScenarioViewModel::RemoveBookmark(uint32 BookmarkId)
{
	uint32 Code = 0;
	return Finish(kod_scenario_remove_bookmark(Handle, BookmarkId, &Code), Code);
}

bool FKodScenarioViewModel::Confirm()
{
	uint32 Code = 0;
	return Finish(kod_scenario_confirm(Handle, &Code), Code);
}

bool FKodScenarioViewModel::Save(TArray<uint8>& OutBytes) const
{
	size_t Len = 0;
	if (kod_scenario_save(Handle, nullptr, 0, &Len) != KOD_STATUS_BUFFER_TOO_SMALL)
	{
		return false;
	}
	OutBytes.SetNumUninitialized(static_cast<int32>(Len));
	return kod_scenario_save(Handle, OutBytes.GetData(), Len, &Len) == KOD_STATUS_OK;
}

bool FKodScenarioViewModel::Load(const TArray<uint8>& Bytes)
{
	KodScenario* Loaded = nullptr;
	if (kod_scenario_load(KOD_ABI_VERSION, Bytes.GetData(), static_cast<size_t>(Bytes.Num()), &Loaded) != KOD_STATUS_OK)
	{
		LastError = FText::FromString(FKodSimulationHandle::LastError());
		Changed.Broadcast();
		return false;
	}
	kod_scenario_destroy(Handle);
	Handle = Loaded;
	LastError = FText::GetEmpty();
	Refresh();
	return true;
}

FText FKodScenarioViewModel::QualityText(const FString& Quality)
{
	if (Quality == TEXT("documented")) return LOCTEXT("QDocumented", "Belegt");
	if (Quality == TEXT("reconstructed")) return LOCTEXT("QReconstructed", "Rekonstruiert");
	if (Quality == TEXT("approximate")) return LOCTEXT("QApproximate", "Näherung");
	if (Quality == TEXT("synthetic")) return LOCTEXT("QSynthetic", "Synthetisch (Testdaten)");
	return LOCTEXT("QNone", "Keine Daten");
}

FText FKodScenarioViewModel::ModeText(const FString& Mode)
{
	if (Mode == TEXT("historical")) return LOCTEXT("MHistorical", "Historisch");
	if (Mode == TEXT("alternate")) return LOCTEXT("MAlternate", "Alternative Geschichte");
	return LOCTEXT("MSandbox", "Sandbox");
}

#undef LOCTEXT_NAMESPACE
