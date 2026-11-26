#pragma once

#include "Modules/ModuleManager.h"

class FKingsOfDawnModule final : public FDefaultGameModuleImpl
{
public:
	void StartupModule() override;
	void ShutdownModule() override;
};
