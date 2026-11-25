# PowerShell 5.1+ / 7. Run with authenticated GitHub CLI.
# Creates the 42 native version milestones, moves existing feature issues,
# creates missing priority labels. Idempotent; does not delete issues or milestones.
$ErrorActionPreference = 'Stop'
$Repo = 'gigafrosxh/kings-of-dawn'
if (-not (Get-Command gh -ErrorAction SilentlyContinue)) { throw 'Install GitHub CLI: winget install --id GitHub.cli -e' }
gh auth status
if ($LASTEXITCODE -ne 0) { throw 'Run gh auth login first.' }

function Invoke-GhApi {
    param([string[]]$Arguments)
    $result = & gh api @Arguments
    if ($LASTEXITCODE -ne 0) { throw "GitHub API failed: $($Arguments -join ' ')" }
    return $result
}

$Plans = @(
    @{ Title = 'v0.1'; Description = 'F-01 - Startjahr und Szenarien. Feature release .0; P0-P4 patches .1-.5; .6+ until stable. See INFO.md.'; Issues = @(1, 6, 7, 8, 9, 10, 11) }
    @{ Title = 'v0.2'; Description = 'F-02 - Historischer Atlas und Kartenimport. Feature release .0; P0-P4 patches .1-.5; .6+ until stable. See INFO.md.'; Issues = @(12) }
    @{ Title = 'v0.3'; Description = 'F-03 - Weltkarte und Kartenmodi. Feature release .0; P0-P4 patches .1-.5; .6+ until stable. See INFO.md.'; Issues = @(13) }
    @{ Title = 'v0.4'; Description = 'F-04 - Geografie- und Provinzeditor. Feature release .0; P0-P4 patches .1-.5; .6+ until stable. See INFO.md.'; Issues = @(14) }
    @{ Title = 'v0.5'; Description = 'F-05 - Staaten, Territorien und nationale Identitaet. Feature release .0; P0-P4 patches .1-.5; .6+ until stable. See INFO.md.'; Issues = @(15) }
    @{ Title = 'v0.6'; Description = 'F-06 - Flaggen, Wappen, Hymnen und Symbolik. Feature release .0; P0-P4 patches .1-.5; .6+ until stable. See INFO.md.'; Issues = @(16) }
    @{ Title = 'v0.7'; Description = 'F-08 - Personen, Aemter und Charaktereditor. Feature release .0; P0-P4 patches .1-.5; .6+ until stable. See INFO.md.'; Issues = @(18) }
    @{ Title = 'v0.8'; Description = 'F-07 - Verfassung und Regierungsform. Feature release .0; P0-P4 patches .1-.5; .6+ until stable. See INFO.md.'; Issues = @(17) }
    @{ Title = 'v0.9'; Description = 'F-12 - Verwaltung, Ministerien und oeffentliche Dienste. Feature release .0; P0-P4 patches .1-.5; .6+ until stable. See INFO.md.'; Issues = @(22) }
    @{ Title = 'v0.10'; Description = 'F-10 - Gesetzgebung und Gesetzeseditor. Feature release .0; P0-P4 patches .1-.5; .6+ until stable. See INFO.md.'; Issues = @(20) }
    @{ Title = 'v0.11'; Description = 'F-15 - Staatshaushalt, Steuern und Staatsverschuldung. Feature release .0; P0-P4 patches .1-.5; .6+ until stable. See INFO.md.'; Issues = @(25) }
    @{ Title = 'v0.12'; Description = 'F-19 - Bevoelkerung, Demografie und Migration. Feature release .0; P0-P4 patches .1-.5; .6+ until stable. See INFO.md.'; Issues = @(29) }
    @{ Title = 'v0.13'; Description = 'F-16 - Waren, Maerkte und Preissimulation. Feature release .0; P0-P4 patches .1-.5; .6+ until stable. See INFO.md.'; Issues = @(26) }
    @{ Title = 'v0.14'; Description = 'F-21 - Staedtebau, Wohnraum und Gebaeude. Feature release .0; P0-P4 patches .1-.5; .6+ until stable. See INFO.md.'; Issues = @(31) }
    @{ Title = 'v0.15'; Description = 'F-17 - Unternehmen, Produktion und Lieferketten. Feature release .0; P0-P4 patches .1-.5; .6+ until stable. See INFO.md.'; Issues = @(27) }
    @{ Title = 'v0.16'; Description = 'F-14 - Waehrungen und Zentralbank. Feature release .0; P0-P4 patches .1-.5; .6+ until stable. See INFO.md.'; Issues = @(24) }
    @{ Title = 'v0.17'; Description = 'F-22 - Verkehr, Energie und Versorgung. Feature release .0; P0-P4 patches .1-.5; .6+ until stable. See INFO.md.'; Issues = @(32) }
    @{ Title = 'v0.18'; Description = 'F-27 - Beziehungen, Anerkennung und Verhandlungen. Feature release .0; P0-P4 patches .1-.5; .6+ until stable. See INFO.md.'; Issues = @(37) }
    @{ Title = 'v0.19'; Description = 'F-35 - Autonome KI und Entscheidungssystem. Feature release .0; P0-P4 patches .1-.5; .6+ until stable. See INFO.md.'; Issues = @(45) }
    @{ Title = 'v0.20'; Description = 'F-39 - Spielmodi, Zeitsteuerung und Speichern. Feature release .0; P0-P4 patches .1-.5; .6+ until stable. See INFO.md.'; Issues = @(49) }
    @{ Title = 'v0.21'; Description = 'F-42 - UX, Einstellungen und Onboarding. Feature release .0; P0-P4 patches .1-.5; .6+ until stable. See INFO.md.'; Issues = @(52) }
    @{ Title = 'v0.22'; Description = 'F-38 - Statistiken, Ranglisten und Analyse. Feature release .0; P0-P4 patches .1-.5; .6+ until stable. See INFO.md.'; Issues = @(48) }
    @{ Title = 'v0.23'; Description = 'F-09 - Dynastien, Erbfolge und Biografien. Feature release .0; P0-P4 patches .1-.5; .6+ until stable. See INFO.md.'; Issues = @(19) }
    @{ Title = 'v0.24'; Description = 'F-11 - Parteien, Ideologien und Wahlen. Feature release .0; P0-P4 patches .1-.5; .6+ until stable. See INFO.md.'; Issues = @(21) }
    @{ Title = 'v0.25'; Description = 'F-13 - Justiz, Sicherheit und Buergerrechte. Feature release .0; P0-P4 patches .1-.5; .6+ until stable. See INFO.md.'; Issues = @(23) }
    @{ Title = 'v0.26'; Description = 'F-20 - Kultur, Religion und Sprache. Feature release .0; P0-P4 patches .1-.5; .6+ until stable. See INFO.md.'; Issues = @(30) }
    @{ Title = 'v0.27'; Description = 'F-23 - Bildung, Gesundheit und Sozialstaat. Feature release .0; P0-P4 patches .1-.5; .6+ until stable. See INFO.md.'; Issues = @(33) }
    @{ Title = 'v0.28'; Description = 'F-24 - Oeffentliche Meinung, Medien und Unruhen. Feature release .0; P0-P4 patches .1-.5; .6+ until stable. See INFO.md.'; Issues = @(34) }
    @{ Title = 'v0.29'; Description = 'F-25 - Umwelt, Klima und Landwirtschaft. Feature release .0; P0-P4 patches .1-.5; .6+ until stable. See INFO.md.'; Issues = @(35) }
    @{ Title = 'v0.30'; Description = 'F-26 - Forschung, Erfindungen und Wissenstransfer. Feature release .0; P0-P4 patches .1-.5; .6+ until stable. See INFO.md.'; Issues = @(36) }
    @{ Title = 'v0.31'; Description = 'F-28 - Vertraege, Buendnisse und internationale Organisationen. Feature release .0; P0-P4 patches .1-.5; .6+ until stable. See INFO.md.'; Issues = @(38) }
    @{ Title = 'v0.32'; Description = 'F-18 - Handel, Banken und Kapitalmarkt. Feature release .0; P0-P4 patches .1-.5; .6+ until stable. See INFO.md.'; Issues = @(28) }
    @{ Title = 'v0.33'; Description = 'F-29 - Streitkraefte, Organisation und Personal. Feature release .0; P0-P4 patches .1-.5; .6+ until stable. See INFO.md.'; Issues = @(39) }
    @{ Title = 'v0.34'; Description = 'F-30 - Ausruestung, Ruestung und Militaerdoktrin. Feature release .0; P0-P4 patches .1-.5; .6+ until stable. See INFO.md.'; Issues = @(40) }
    @{ Title = 'v0.35'; Description = 'F-32 - Nachschub, Transport und Mobilisierung. Feature release .0; P0-P4 patches .1-.5; .6+ until stable. See INFO.md.'; Issues = @(42) }
    @{ Title = 'v0.36'; Description = 'F-31 - Krieg, Fronten und Konfliktsimulation. Feature release .0; P0-P4 patches .1-.5; .6+ until stable. See INFO.md.'; Issues = @(41) }
    @{ Title = 'v0.37'; Description = 'F-33 - Besatzung, Friedenskonferenz und Wiederaufbau. Feature release .0; P0-P4 patches .1-.5; .6+ until stable. See INFO.md.'; Issues = @(43) }
    @{ Title = 'v0.38'; Description = 'F-34 - Nachrichtendienste und Informationslage. Feature release .0; P0-P4 patches .1-.5; .6+ until stable. See INFO.md.'; Issues = @(44) }
    @{ Title = 'v0.39'; Description = 'F-36 - Ereignisse, Missionen und Szenarioeditor. Feature release .0; P0-P4 patches .1-.5; .6+ until stable. See INFO.md.'; Issues = @(46) }
    @{ Title = 'v0.40'; Description = 'F-37 - Zeitung, Chronik und Geschichtsarchiv. Feature release .0; P0-P4 patches .1-.5; .6+ until stable. See INFO.md.'; Issues = @(47) }
    @{ Title = 'v0.41'; Description = 'F-40 - Modding und benutzerdefinierte Inhalte. Feature release .0; P0-P4 patches .1-.5; .6+ until stable. See INFO.md.'; Issues = @(50) }
    @{ Title = 'v0.42'; Description = 'F-41 - Multiplayer und Rollenverwaltung. Feature release .0; P0-P4 patches .1-.5; .6+ until stable. See INFO.md.'; Issues = @(51) }
)

$Labels = @(
    @{ Name = 'bug'; Color = 'd73a4a'; Description = 'Confirmed or reported defect' }
    @{ Name = 'P0'; Color = 'B60205'; Description = 'Critical blocker, crash, data loss or security' }
    @{ Name = 'P1'; Color = 'D93F0B'; Description = 'Severe functional regression or unusable core feature' }
    @{ Name = 'P2'; Color = 'FBCA04'; Description = 'Moderate functional defect' }
    @{ Name = 'P3'; Color = '0E8A16'; Description = 'Minor functional or UI defect' }
    @{ Name = 'P4'; Color = 'C5DEF5'; Description = 'Cosmetic defect' }
)
$ExistingLabels = @(Invoke-GhApi -Arguments @('--paginate', "repos/$Repo/labels?per_page=100", '--jq', '.[].name'))
foreach ($Label in $Labels) {
    if ($ExistingLabels -notcontains $Label.Name) {
        $null = Invoke-GhApi -Arguments @('--method','POST',"repos/$Repo/labels",'-f',"name=$($Label.Name)",'-f',"color=$($Label.Color)",'-f',"description=$($Label.Description)")
        Write-Host "Created label $($Label.Name)"
    }
}

$Existing = @(Invoke-GhApi -Arguments @('--paginate', "repos/$Repo/milestones?state=all&per_page=100", '--jq', '.[] | @json') | ForEach-Object { $_ | ConvertFrom-Json })
foreach ($Plan in $Plans) {
    $Match = $Existing | Where-Object { $_.title -eq $Plan.Title } | Select-Object -First 1
    if ($null -eq $Match) {
        $Number = [int](Invoke-GhApi -Arguments @('--method','POST',"repos/$Repo/milestones",'-f',"title=$($Plan.Title)",'-f',"description=$($Plan.Description)",'--jq','.number'))
    } else {
        $Number = [int]$Match.number
        # Preserve state, dates and manually edited descriptions.
    }
    foreach ($Issue in $Plan.Issues) {
        $Current = Invoke-GhApi -Arguments @("repos/$Repo/issues/$Issue",'--jq','.milestone.number // 0')
        if ([int]$Current -ne $Number) {
            if ($null -ne $Match -and $Match.state -eq 'closed') { throw "Closed target milestone $($Plan.Title) has an unassigned issue #$Issue; review manually." }
            $null = Invoke-GhApi -Arguments @('--method','PATCH',"repos/$Repo/issues/$Issue",'-F',"milestone=$Number",'--jq','.number')
        }
    }
    Write-Host "$($Plan.Title): $($Plan.Issues.Count) planned issues assigned (milestone #$Number)"
}

# The former M1-M4 tracking issues remain historical indexes.
# Remove them from legacy phase milestones only, not from unrelated milestones.
$Legacy = @($Existing | Where-Object { $_.title -match '^M[0-4] ' })
foreach ($Issue in @(2,3,4,5)) {
    $Current = [int](Invoke-GhApi -Arguments @("repos/$Repo/issues/$Issue",'--jq','.milestone.number // 0'))
    if ($Current -ne 0 -and $Current -in @($Legacy | ForEach-Object { [int]$_.number })) {
        $null = Invoke-GhApi -Arguments @('--method','PATCH',"repos/$Repo/issues/$Issue",'-F','milestone=null','--jq','.number')
    }
}
# Close only empty former milestones; never move unknown bugs or delete history.
foreach ($Old in $Legacy) {
    $Fresh = Invoke-GhApi -Arguments @("repos/$Repo/milestones/$($Old.number)") | ConvertFrom-Json
    if ($Fresh.state -eq 'open' -and $Fresh.open_issues -eq 0) {
        $null = Invoke-GhApi -Arguments @('--method','PATCH',"repos/$Repo/milestones/$($Old.number)",'-f','state=closed','--jq','.number')
        Write-Host "Archived empty phase milestone $($Old.title)"
    } elseif ($Fresh.open_issues -gt 0) {
        Write-Warning "Legacy milestone $($Old.title) still contains $($Fresh.open_issues) open issues; review and assign manually."
    }
}
Write-Host "https://github.com/$Repo/milestones"
