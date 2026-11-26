#pragma once

#include "CoreMinimal.h"
#include "GameFramework/GameModeBase.h"
#include "GameFramework/Pawn.h"
#include "MapPrototype.generated.h"

class UCameraComponent;
class UInstancedStaticMeshComponent;

UCLASS()
class AMapPrototypePawn : public APawn
{
    GENERATED_BODY()
public:
    AMapPrototypePawn();
    virtual void BeginPlay() override;
    virtual void Tick(float DeltaSeconds) override;
    static int32 CellAt(const FVector& Local);
private:
    UPROPERTY() TObjectPtr<UCameraComponent> Camera;
    UPROPERTY() TArray<TObjectPtr<UInstancedStaticMeshComponent>> TileLayers;
    bool bBenchmark = false;
    int32 BenchmarkFrame = 0;
    double BenchmarkStart = 0;
    bool bOverview = false;
};

UCLASS()
class AMapPrototypeMode : public AGameModeBase
{
    GENERATED_BODY()
public:
    AMapPrototypeMode();
};
