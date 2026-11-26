using UnrealBuildTool;

public class KingsOfDawnSimulation : ModuleRules
{
	public KingsOfDawnSimulation(ReadOnlyTargetRules Target) : base(Target)
	{
		PCHUsage = PCHUsageMode.UseExplicitOrSharedPCHs;
		PublicDependencyModuleNames.AddRange(new[] { "Core", "CoreUObject", "Engine" });
		PrivateDependencyModuleNames.AddRange(new[] { "KodFfi" });
	}
}
