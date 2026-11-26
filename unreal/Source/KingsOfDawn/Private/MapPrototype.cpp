#include "MapPrototype.h"
#include "Camera/CameraComponent.h"
#include "Components/DirectionalLightComponent.h"
#include "Misc/CommandLine.h"
#include "Misc/Parse.h"
#include "Components/InstancedStaticMeshComponent.h"
#include "Engine/Engine.h"
#include "Engine/StaticMesh.h"
#include "GameFramework/PlayerController.h"
#include "InputCoreTypes.h"
#include "Materials/MaterialInstanceDynamic.h"
#include "UObject/ConstructorHelpers.h"

namespace
{
constexpr int32 Side = 100;
constexpr int32 CellSize = 100;
constexpr int32 Colors = 4;
constexpr int32 HalfWidth = Side * CellSize / 2;
}

AMapPrototypeMode::AMapPrototypeMode()
{
    DefaultPawnClass = AMapPrototypePawn::StaticClass();
}

AMapPrototypePawn::AMapPrototypePawn()
{
    PrimaryActorTick.bCanEverTick = true;
    RootComponent = CreateDefaultSubobject<USceneComponent>(TEXT("Map"));
    auto* Sun = CreateDefaultSubobject<UDirectionalLightComponent>(TEXT("MapLight"));
    Sun->SetupAttachment(RootComponent);
    Sun->SetRelativeRotation(FRotator(-80, 0, 0));
    Sun->SetIntensity(4.f);
    Camera = CreateDefaultSubobject<UCameraComponent>(TEXT("Camera"));
    Camera->SetupAttachment(RootComponent);
    Camera->SetRelativeLocation(FVector(0,0,8000));
    Camera->SetRelativeRotation(FRotator(-90,0,0));
    Camera->ProjectionMode = ECameraProjectionMode::Orthographic;
    Camera->OrthoWidth = 19000;
    static ConstructorHelpers::FObjectFinder<UStaticMesh> Tile(TEXT("/Engine/BasicShapes/Cube.Cube"));
    for (int32 Index = 0; Index < Colors * 2; ++Index)
    {
        auto* Layer = CreateDefaultSubobject<UInstancedStaticMeshComponent>(*FString::Printf(TEXT("Layer%d"), Index));
        Layer->SetupAttachment(RootComponent);
        Layer->SetStaticMesh(Tile.Object);
        Layer->SetCollisionEnabled(ECollisionEnabled::NoCollision);
        Layer->SetCastShadow(false);
        TileLayers.Add(Layer);
    }
}

void AMapPrototypePawn::BeginPlay()
{
    Super::BeginPlay();
    bBenchmark = FParse::Param(FCommandLine::Get(), TEXT("KodMapBenchmark"));
    if (auto* PC = Cast<APlayerController>(Controller)) PC->bShowMouseCursor = true;
    const FLinearColor Palette[] = {FLinearColor::Red, FLinearColor::Green, FLinearColor::Blue, FLinearColor::Yellow};
    for (int32 Level = 0; Level < 2; ++Level)
    {
        const int32 Stride = Level == 0 ? 1 : 5;
        TArray<FTransform> Groups[Colors];
        for (int32 Y = 0; Y < Side; Y += Stride)
        {
            for (int32 X = 0; X < Side; X += Stride)
            {
                const int32 Owner = (X / 10 + 3 * (Y / 10)) % Colors;
                const FVector Center((X + Stride * .5) * CellSize - HalfWidth, (Y + Stride * .5) * CellSize - HalfWidth, 0);
                Groups[Owner].Emplace(FQuat::Identity, Center, FVector(Stride * .98, Stride * .98, .01));
            }
        }
        for (int32 Owner = 0; Owner < Colors; ++Owner)
        {
            auto* Layer = TileLayers[Level * Colors + Owner].Get();
            if (auto* Material = Layer->CreateDynamicMaterialInstance(0))
                Material->SetVectorParameterValue(TEXT("Color"), Palette[Owner]);
            Layer->AddInstances(Groups[Owner], false, false, false);
            Layer->SetVisibility(Level == 0);
        }
    }
}

int32 AMapPrototypePawn::CellAt(const FVector& Local)
{
    const int32 X = FMath::FloorToInt((Local.X + HalfWidth) / CellSize);
    const int32 Y = FMath::FloorToInt((Local.Y + HalfWidth) / CellSize);
    return X >= 0 && X < Side && Y >= 0 && Y < Side ? Y * Side + X : INDEX_NONE;
}

void AMapPrototypePawn::Tick(float DeltaSeconds)
{
    Super::Tick(DeltaSeconds);
    if (bBenchmark)
    {
        ++BenchmarkFrame;
        if (BenchmarkFrame == 120) BenchmarkStart = FPlatformTime::Seconds();
        if (BenchmarkFrame == 420)
        {
            const double Elapsed = FPlatformTime::Seconds() - BenchmarkStart;
            UE_LOG(LogTemp, Display, TEXT("Map prototype: 300 frames, %.2f ms/frame (wall clock)"), Elapsed * 1000.0 / 300);
            FPlatformMisc::RequestExit(false);
        }
    }
    auto* PC = Cast<APlayerController>(Controller);
    if (!PC) return;
    PC->bShowMouseCursor = true;
    const float Wheel = PC->GetInputAnalogKeyState(EKeys::MouseWheelAxis);
    if (Wheel != 0)
        Camera->SetOrthoWidth(FMath::Clamp(Camera->OrthoWidth * FMath::Pow(.8f, Wheel), 1000.f, 40000.f));
    const bool bFar = Camera->OrthoWidth > 20000;
    if (bOverview != bFar)
    {
        bOverview = bFar;
        for (int32 Index = 0; Index < TileLayers.Num(); ++Index)
            TileLayers[Index]->SetVisibility((Index >= Colors) == bFar);
    }
    if (PC->WasInputKeyJustPressed(EKeys::LeftMouseButton))
    {
        FVector Origin, Direction;
        if (PC->DeprojectMousePositionToWorld(Origin, Direction))
        {
            const FTransform Transform = GetActorTransform();
            Origin = Transform.InverseTransformPosition(Origin);
            Direction = Transform.InverseTransformVector(Direction);
            if (FMath::IsNearlyZero(Direction.Z)) return;
            const int32 Id = CellAt(Origin - Direction * (Origin.Z / Direction.Z));
            if (Id != INDEX_NONE && GEngine)
                GEngine->AddOnScreenDebugMessage(1, 4.f, FColor::White, FString::Printf(TEXT("Cell %u"), static_cast<uint32>(Id)));
        }
    }
}

#if WITH_DEV_AUTOMATION_TESTS
#include "Misc/AutomationTest.h"

IMPLEMENT_SIMPLE_AUTOMATION_TEST(FMapPickingTest, "KingsOfDawn.Map.Picking",
    EAutomationTestFlags::EditorContext | EAutomationTestFlags::EngineFilter)

bool FMapPickingTest::RunTest(const FString&)
{
    for (int32 Id = 0; Id < Side * Side; ++Id)
    {
        const FVector Center((Id % Side + .5) * CellSize - HalfWidth, (Id / Side + .5) * CellSize - HalfWidth, 0);
        if (!TestEqual(TEXT("Cell center"), AMapPrototypePawn::CellAt(Center), Id)) return false;
    }
    TestEqual(TEXT("Upper border excluded"), AMapPrototypePawn::CellAt(FVector(HalfWidth, 0, 0)), INDEX_NONE);
    TestEqual(TEXT("Lower border included"), AMapPrototypePawn::CellAt(FVector(-HalfWidth, -HalfWidth, 0)), 0);
    return true;
}
#endif
