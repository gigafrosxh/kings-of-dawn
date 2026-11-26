using UnrealBuildTool;
using System.Collections.Generic;

public class KingsOfDawnEditorTarget : TargetRules
{
	public KingsOfDawnEditorTarget(TargetInfo Target) : base(Target)
	{
		Type = TargetType.Editor;
		DefaultBuildSettings = BuildSettingsVersion.V7;
		IncludeOrderVersion = EngineIncludeOrderVersion.Unreal5_8;
		ExtraModuleNames.Add("KingsOfDawn");
	}
}
