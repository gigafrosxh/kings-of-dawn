#include "SKodScenarioPanel.h"

#include "KodScenarioViewModel.h"
#include "Styling/CoreStyle.h"
#include "Widgets/Input/SButton.h"
#include "Widgets/Input/SEditableTextBox.h"
#include "Widgets/Layout/SBorder.h"
#include "Widgets/Layout/SBox.h"
#include "Widgets/Layout/SScrollBox.h"
#include "Widgets/Layout/SUniformGridPanel.h"
#include "Widgets/SBoxPanel.h"
#include "Widgets/Text/STextBlock.h"

#define LOCTEXT_NAMESPACE "KodScenario"

namespace KodStyle
{
	// "Modernes HOI4": dark steel panels, warm brass/gold accents.
	const FLinearColor Panel(0.012f, 0.016f, 0.018f, 0.96f);
	const FLinearColor Section(0.035f, 0.042f, 0.046f, 1.f);
	const FLinearColor Header(0.06f, 0.07f, 0.075f, 1.f);
	const FLinearColor Button(0.045f, 0.052f, 0.058f, 1.f);
	const FLinearColor Brass(0.58f, 0.40f, 0.16f, 1.f);
	const FLinearColor BrassDim(0.30f, 0.21f, 0.09f, 1.f);
	const FLinearColor Text(0.86f, 0.80f, 0.66f, 1.f);
	const FLinearColor TextDim(0.55f, 0.53f, 0.48f, 1.f);
	const FLinearColor Warning(0.85f, 0.45f, 0.18f, 1.f);

	FSlateFontInfo Font(int32 Size, bool bBold = false)
	{
		return FCoreStyle::GetDefaultFontStyle(bBold ? "Bold" : "Regular", Size);
	}

	const FSlateBrush* White() { return FCoreStyle::Get().GetBrush("WhiteBrush"); }
}

void SKodScenarioPanel::Construct(const FArguments& InArgs)
{
	ViewModel = InArgs._ViewModel;
	OnStarted = InArgs._OnStarted;
	if (ViewModel.IsValid())
	{
		ChangedHandle = ViewModel->OnChanged().AddSP(this, &SKodScenarioPanel::MarkDirty);
	}

	ChildSlot
	[
		SNew(SBox)
		.WidthOverride(430.f)
		[
			SNew(SBorder)
			.BorderImage(KodStyle::White())
			.BorderBackgroundColor(KodStyle::BrassDim)
			.Padding(1.f)
			[
				SNew(SBorder)
				.BorderImage(KodStyle::White())
				.BorderBackgroundColor(KodStyle::Panel)
				.Padding(8.f)
				[
					SNew(SScrollBox)
					+ SScrollBox::Slot()
					[
						SAssignNew(Content, SVerticalBox)
					]
				]
			]
		]
	];
	Rebuild();
}

SKodScenarioPanel::~SKodScenarioPanel()
{
	if (ViewModel.IsValid())
	{
		ViewModel->OnChanged().Remove(ChangedHandle);
	}
}

TSharedRef<SWidget> SKodScenarioPanel::MakeButton(const FText& Label, FOnClicked OnClicked, bool bActive, bool bEnabled) const
{
	return SNew(SBorder)
		.BorderImage(KodStyle::White())
		.BorderBackgroundColor(bActive ? KodStyle::Brass : KodStyle::BrassDim)
		.Padding(1.f)
		[
			SNew(SButton)
			.ButtonColorAndOpacity(bActive ? KodStyle::Header : KodStyle::Button)
			.IsEnabled(bEnabled)
			.HAlign(HAlign_Center)
			.VAlign(VAlign_Center)
			.ContentPadding(FMargin(6.f, 6.f))
			.OnClicked(OnClicked)
			[
				SNew(STextBlock)
				.Text(Label)
				.Font(KodStyle::Font(10, true))
				.ColorAndOpacity(bActive ? KodStyle::Brass : KodStyle::Text)
			]
		];
}

TSharedRef<SWidget> SKodScenarioPanel::BuildSection(const FText& Title, const TSharedRef<SWidget>& Body) const
{
	return SNew(SVerticalBox)
		+ SVerticalBox::Slot().AutoHeight().Padding(0.f, 6.f, 0.f, 2.f)
		[
			SNew(SBorder)
			.BorderImage(KodStyle::White())
			.BorderBackgroundColor(KodStyle::Header)
			.Padding(FMargin(10.f, 6.f))
			[
				SNew(STextBlock).Text(Title).Font(KodStyle::Font(12, true)).ColorAndOpacity(KodStyle::Brass)
			]
		]
		+ SVerticalBox::Slot().AutoHeight()
		[
			SNew(SBorder)
			.BorderImage(KodStyle::White())
			.BorderBackgroundColor(KodStyle::Section)
			.Padding(6.f)
			[
				Body
			]
		];
}

TSharedRef<SWidget> SKodScenarioPanel::BuildTabs()
{
	const FString& Mode = ViewModel->GetView().Mode;
	auto Tab = [this, &Mode](ETab T, const FText& Label)
	{
		return MakeButton(Label, FOnClicked::CreateSP(this, &SKodScenarioPanel::OnTab, T), ActiveTab == T);
	};
	TSharedRef<SUniformGridPanel> Grid = SNew(SUniformGridPanel).SlotPadding(FMargin(2.f));
	Grid->AddSlot(0, 0)[Tab(ETab::Bookmarks, LOCTEXT("TabBookmarks", "Lesezeichen"))];
	Grid->AddSlot(1, 0)[Tab(ETab::Historical, LOCTEXT("TabHistorical", "Historisch"))];
	Grid->AddSlot(2, 0)[Tab(ETab::Alternate, LOCTEXT("TabAlternate", "Alternativ"))];
	Grid->AddSlot(3, 0)[Tab(ETab::Sandbox, LOCTEXT("TabSandbox", "Sandbox"))];
	return Grid;
}

TSharedRef<SWidget> SKodScenarioPanel::BuildYearGrid()
{
	const FKodScenarioView& V = ViewModel->GetView();
	const bool bLocked = V.bConfirmed;
	TSharedRef<SUniformGridPanel> Grid = SNew(SUniformGridPanel).SlotPadding(FMargin(3.f));
	for (int32 i = 0; i < V.Presets.Num(); ++i)
	{
		const int32 Year = V.Presets[i].Key;
		const FText Label = (i == V.Presets.Num() - 1) ? LOCTEXT("Today", "Heute") : FText::FromString(V.Presets[i].Value);
		Grid->AddSlot(i % 5, i / 5)[MakeButton(Label, FOnClicked::CreateSP(this, &SKodScenarioPanel::OnPreset, Year), V.Year == Year, !bLocked)];
	}

	TSharedRef<SHorizontalBox> Stepper = SNew(SHorizontalBox);
	for (int32 Delta : {-100, -10, -1})
	{
		Stepper->AddSlot().FillWidth(1.f).Padding(2.f)[MakeButton(FText::AsNumber(Delta), FOnClicked::CreateSP(this, &SKodScenarioPanel::OnStep, Delta), false, !bLocked)];
	}
	Stepper->AddSlot().FillWidth(1.6f).VAlign(VAlign_Center).HAlign(HAlign_Center)
	[
		SNew(STextBlock).Text(FText::FromString(V.YearLabel)).Font(KodStyle::Font(14, true)).ColorAndOpacity(KodStyle::Brass)
	];
	for (int32 Delta : {1, 10, 100})
	{
		Stepper->AddSlot().FillWidth(1.f).Padding(2.f)[MakeButton(FText::FromString(FString::Printf(TEXT("+%d"), Delta)), FOnClicked::CreateSP(this, &SKodScenarioPanel::OnStep, Delta), false, !bLocked)];
	}

	return SNew(SVerticalBox)
		+ SVerticalBox::Slot().AutoHeight()[Grid]
		+ SVerticalBox::Slot().AutoHeight().Padding(0.f, 6.f, 0.f, 0.f)[Stepper];
}

TSharedRef<SWidget> SKodScenarioPanel::BuildPolities()
{
	const FKodScenarioView& V = ViewModel->GetView();
	TSharedRef<SVerticalBox> List = SNew(SVerticalBox);
	if (V.Polities.Num() == 0)
	{
		List->AddSlot().AutoHeight().Padding(4.f)
		[
			SNew(STextBlock)
			.Text(LOCTEXT("NoPolities", "Für dieses Jahr liegen keine Staaten im Datensatz vor."))
			.ColorAndOpacity(KodStyle::Warning).AutoWrapText(true)
		];
	}
	for (const FKodPolityItem& P : V.Polities)
	{
		List->AddSlot().AutoHeight().Padding(0.f, 1.f)
		[
			SNew(SHorizontalBox)
			+ SHorizontalBox::Slot().FillWidth(1.f)
			[
				MakeButton(FText::FromString(P.Name), FOnClicked::CreateSP(this, &SKodScenarioPanel::OnPolity, P.Id), V.SelectedPolity == P.Id, !V.bConfirmed)
			]
			+ SHorizontalBox::Slot().AutoWidth().VAlign(VAlign_Center).Padding(8.f, 0.f, 2.f, 0.f)
			[
				SNew(STextBlock).Text(FKodScenarioViewModel::QualityText(P.Quality)).Font(KodStyle::Font(9)).ColorAndOpacity(KodStyle::TextDim)
			]
		];
	}
	return List;
}

FText SKodScenarioPanel::SelectedPolityName() const
{
	const FKodScenarioView& V = ViewModel->GetView();
	for (const FKodPolityItem& P : V.Polities)
	{
		if (P.Id == V.SelectedPolity)
		{
			return FText::FromString(P.Name);
		}
	}
	return LOCTEXT("NoneSelected", "– nicht gewählt –");
}

FText SKodScenarioPanel::DataStatusText() const
{
	const FKodScenarioView& V = ViewModel->GetView();
	if (V.StatusKind == TEXT("missing"))
	{
		return LOCTEXT("StatusMissing", "Keine Daten für dieses Jahr");
	}
	if (V.StatusKind == TEXT("partial"))
	{
		return FText::Format(LOCTEXT("StatusPartial", "Lückenhaft: {0}"), FText::FromString(V.StatusReason));
	}
	return LOCTEXT("StatusComplete", "Vollständig im Datensatz");
}

TArray<FText> SKodScenarioPanel::GetDetailTexts() const
{
	const FKodScenarioView& V = ViewModel->GetView();
	return {
		FText::FromString(V.YearLabel),
		FKodScenarioViewModel::ModeText(V.Mode),
		SelectedPolityName(),
		FKodScenarioViewModel::QualityText(V.MinQuality),
		DataStatusText(),
	};
}

TSharedRef<SWidget> SKodScenarioPanel::BuildDetails()
{
	const FKodScenarioView& V = ViewModel->GetView();
	const TArray<FText> Values = GetDetailTexts();
	const FText Labels[] = {
		LOCTEXT("DYear", "Startjahr"), LOCTEXT("DMode", "Spielmodus"), LOCTEXT("DState", "Land"),
		LOCTEXT("DQuality", "Quellengüte"), LOCTEXT("DStatus", "Datenstand"),
	};
	TSharedRef<SVerticalBox> Rows = SNew(SVerticalBox);
	auto Row = [&Rows](const FText& Label, const FText& Value, const FLinearColor& Color)
	{
		Rows->AddSlot().AutoHeight().Padding(4.f, 3.f)
		[
			SNew(SHorizontalBox)
			+ SHorizontalBox::Slot().FillWidth(1.f)[SNew(STextBlock).Text(Label).Font(KodStyle::Font(10)).ColorAndOpacity(KodStyle::TextDim)]
			+ SHorizontalBox::Slot().AutoWidth()[SNew(STextBlock).Text(Value).Font(KodStyle::Font(10, true)).ColorAndOpacity(Color)]
		];
	};
	Row(Labels[0], Values[0], KodStyle::Text);
	Row(LOCTEXT("DDate", "Startdatum"), LOCTEXT("DDateValue", "1. Januar (Konvention)"), KodStyle::Text);
	for (int32 i = 1; i < Values.Num(); ++i)
	{
		const bool bWarn = (i == 4 && V.StatusKind != TEXT("complete")) || (i == 3 && V.MinQuality == TEXT("synthetic"));
		Row(Labels[i], Values[i], bWarn ? KodStyle::Warning : KodStyle::Text);
	}
	Row(LOCTEXT("DSource", "Quelle"), FText::FromString(FString::Printf(TEXT("%s %s"), *V.DatasetId, *V.DatasetVersion)), KodStyle::TextDim);
	return Rows;
}

TSharedRef<SWidget> SKodScenarioPanel::BuildBookmarks()
{
	const FKodScenarioView& V = ViewModel->GetView();
	TSharedRef<SVerticalBox> List = SNew(SVerticalBox);
	List->AddSlot().AutoHeight().Padding(0.f, 2.f)
	[
		SNew(SHorizontalBox)
		+ SHorizontalBox::Slot().FillWidth(1.f)[SAssignNew(NameBox, SEditableTextBox).HintText(LOCTEXT("BookmarkHint", "Name des Lesezeichens"))]
		+ SHorizontalBox::Slot().AutoWidth().Padding(4.f, 0.f, 0.f, 0.f)[MakeButton(LOCTEXT("AddBookmark", "Speichern"), FOnClicked::CreateSP(this, &SKodScenarioPanel::OnAddBookmark), false, !V.bConfirmed)]
	];
	if (V.Bookmarks.Num() == 0)
	{
		List->AddSlot().AutoHeight().Padding(4.f)[SNew(STextBlock).Text(LOCTEXT("NoBookmarks", "Noch keine Lesezeichen.")).ColorAndOpacity(KodStyle::TextDim)];
	}
	for (const FKodBookmarkItem& B : V.Bookmarks)
	{
		List->AddSlot().AutoHeight().Padding(0.f, 1.f)
		[
			SNew(SHorizontalBox)
			+ SHorizontalBox::Slot().FillWidth(1.f)[MakeButton(FText::FromString(B.Name), FOnClicked::CreateSP(this, &SKodScenarioPanel::OnApplyBookmark, B.Id), false, !V.bConfirmed)]
			+ SHorizontalBox::Slot().AutoWidth().Padding(4.f, 0.f, 0.f, 0.f)[MakeButton(LOCTEXT("Remove", "✕"), FOnClicked::CreateSP(this, &SKodScenarioPanel::OnRemoveBookmark, B.Id), false, !V.bConfirmed)]
		];
	}
	return List;
}

TSharedRef<SWidget> SKodScenarioPanel::BuildSandbox()
{
	const FKodScenarioView& V = ViewModel->GetView();
	return SNew(SHorizontalBox)
		+ SHorizontalBox::Slot().FillWidth(1.f)[SAssignNew(NameBox, SEditableTextBox).HintText(LOCTEXT("CustomHint", "Name eines eigenen Staates"))]
		+ SHorizontalBox::Slot().AutoWidth().Padding(4.f, 0.f, 0.f, 0.f)[MakeButton(LOCTEXT("CreateCustom", "Gründen"), FOnClicked::CreateSP(this, &SKodScenarioPanel::OnCreateCustomPolity), false, V.bCustomPolitiesAllowed && !V.bConfirmed)];
}

void SKodScenarioPanel::Tick(const FGeometry& AllottedGeometry, const double InCurrentTime, const float InDeltaTime)
{
	SCompoundWidget::Tick(AllottedGeometry, InCurrentTime, InDeltaTime);
	// Rebuild outside of click handlers so the clicked button is never destroyed mid-event.
	if (bDirty)
	{
		bDirty = false;
		Rebuild();
	}
}

void SKodScenarioPanel::Rebuild()
{
	if (!Content.IsValid())
	{
		return;
	}
	Content->ClearChildren();
	if (!ViewModel.IsValid() || !ViewModel->IsValid())
	{
		Content->AddSlot().AutoHeight()[SNew(STextBlock).Text(LOCTEXT("NoSim", "Simulation nicht verfügbar.")).ColorAndOpacity(KodStyle::Warning)];
		return;
	}
	const FKodScenarioView& V = ViewModel->GetView();

	Content->AddSlot().AutoHeight().Padding(0.f, 0.f, 0.f, 6.f)
	[
		SNew(STextBlock).Text(LOCTEXT("Title", "Szenario wählen")).Font(KodStyle::Font(14, true)).ColorAndOpacity(KodStyle::Brass)
	];
	Content->AddSlot().AutoHeight()[BuildTabs()];
	if (ActiveTab == ETab::Bookmarks)
	{
		Content->AddSlot().AutoHeight()[BuildSection(LOCTEXT("SBookmarks", "Lesezeichen"), BuildBookmarks())];
	}
	Content->AddSlot().AutoHeight()[BuildSection(LOCTEXT("STime", "Zeit / Auswahl"), BuildYearGrid())];
	Content->AddSlot().AutoHeight()[BuildSection(LOCTEXT("SState", "Land"), BuildPolities())];
	if (ActiveTab == ETab::Sandbox)
	{
		Content->AddSlot().AutoHeight()[BuildSection(LOCTEXT("SSandbox", "Eigener Staat"), BuildSandbox())];
	}
	Content->AddSlot().AutoHeight()[BuildSection(LOCTEXT("SDetails", "Details"), BuildDetails())];
	if (!ViewModel->GetLastError().IsEmpty())
	{
		Content->AddSlot().AutoHeight().Padding(2.f, 6.f)[SNew(STextBlock).Text(ViewModel->GetLastError()).ColorAndOpacity(KodStyle::Warning).AutoWrapText(true)];
	}
	Content->AddSlot().AutoHeight().Padding(0.f, 10.f, 0.f, 0.f)
	[
		MakeButton(V.bConfirmed ? LOCTEXT("Started", "Szenario gestartet") : LOCTEXT("Start", "Szenario starten"),
			FOnClicked::CreateSP(this, &SKodScenarioPanel::OnStart), true, !V.bConfirmed && V.SelectedPolity != 0)
	];
}

FReply SKodScenarioPanel::OnTab(ETab Tab)
{
	ActiveTab = Tab;
	const uint8 Mode = Tab == ETab::Alternate ? 1 : (Tab == ETab::Sandbox ? 2 : 0);
	if (Tab != ETab::Bookmarks)
	{
		ViewModel->SelectMode(Mode); // marks dirty via OnChanged
	}
	MarkDirty();
	return FReply::Handled();
}

FReply SKodScenarioPanel::OnPreset(int32 Year)
{
	ViewModel->SelectYear(Year);
	return FReply::Handled();
}

FReply SKodScenarioPanel::OnStep(int32 Delta)
{
	ViewModel->StepYear(Delta);
	return FReply::Handled();
}

FReply SKodScenarioPanel::OnPolity(uint32 PolityId)
{
	ViewModel->SelectPolity(PolityId);
	return FReply::Handled();
}

FReply SKodScenarioPanel::OnAddBookmark()
{
	const FString Name = NameBox.IsValid() ? NameBox->GetText().ToString() : FString();
	ViewModel->AddBookmark(Name);
	return FReply::Handled();
}

FReply SKodScenarioPanel::OnApplyBookmark(uint32 BookmarkId)
{
	ViewModel->ApplyBookmark(BookmarkId);
	return FReply::Handled();
}

FReply SKodScenarioPanel::OnRemoveBookmark(uint32 BookmarkId)
{
	ViewModel->RemoveBookmark(BookmarkId);
	return FReply::Handled();
}

FReply SKodScenarioPanel::OnCreateCustomPolity()
{
	const FString Name = NameBox.IsValid() ? NameBox->GetText().ToString() : FString();
	ViewModel->CreateCustomPolity(Name);
	return FReply::Handled();
}

FReply SKodScenarioPanel::OnStart()
{
	if (ViewModel->Confirm())
	{
		OnStarted.ExecuteIfBound();
	}
	return FReply::Handled();
}

#undef LOCTEXT_NAMESPACE
