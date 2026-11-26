using UnrealBuildTool;
using System.Collections.Generic;

public class KingsOfDawnTarget : TargetRules
{
	public KingsOfDawnTarget(TargetInfo Target) : base(Target)
	{
		Type = TargetType.Game;
		DefaultBuildSettings = BuildSettingsVersion.V5;
		IncludeOrderVersion = EngineIncludeOrderVersion.Unreal5_8;
		ExtraModuleNames.Add("KingsOfDawn");
	}
}
