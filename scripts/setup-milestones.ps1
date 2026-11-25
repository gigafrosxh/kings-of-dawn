# Run with PowerShell from any directory. Requires GitHub CLI and gh auth login.
# Creates missing native milestones; assigns existing Kings of Dawn issues.
# Safe to run again. Does not close, delete or recreate issues.
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
    @{ Title = 'M0 – Technisches Fundament'; Description = 'UE5, Rust, FFI, CI, Kartenprototyp, Lizenzprüfung.'; Issues = @(1, 6, 7, 8, 9, 10) }
    @{ Title = 'M1 – Spielwelt und Kern-UI'; Description = 'Atlas, Startjahre, Weltkarte, Staaten, HUD und Saves.'; Issues = @(2, 11, 12, 13, 14, 15, 49, 52) }
    @{ Title = 'M2 – Spielbarer Wirtschafts- und Politik-MVP'; Description = 'Wirtschaft, Politik, Städte, Bevölkerung und autonome Staaten.'; Issues = @(3, 16, 17, 18, 20, 22, 24, 25, 26, 27, 29, 31, 32, 37, 45, 48) }
    @{ Title = 'M3 – Grand Strategy V1'; Description = 'Diplomatie, Militär, Technologie, Gesellschaft, Ereignisse und Modding.'; Issues = @(4, 19, 21, 23, 28, 30, 33, 34, 35, 36, 38, 39, 40, 41, 42, 43, 44, 46, 47, 50) }
    @{ Title = 'M4 – Multiplayer und Stabilisierung'; Description = 'Multiplayer, Rollenverwaltung und Release-Abnahme.'; Issues = @(5, 51) }
)
$Existing = @(Invoke-GhApi -Arguments @('--paginate', "repos/$Repo/milestones?state=all&per_page=100", '--jq', '.[] | @json') | ForEach-Object { $_ | ConvertFrom-Json })
foreach ($Plan in $Plans) {
    $Match = $Existing | Where-Object { $_.title -eq $Plan.Title } | Select-Object -First 1
    if ($null -eq $Match) {
        $Number = [int](Invoke-GhApi -Arguments @('--method','POST',"repos/$Repo/milestones",'-f',"title=$($Plan.Title)",'-f',"description=$($Plan.Description)",'--jq','.number'))
    } else {
        $Number = [int]$Match.number
    }
    foreach ($Issue in $Plan.Issues) {
        $null = Invoke-GhApi -Arguments @('--method','PATCH',"repos/$Repo/issues/$Issue",'-F',"milestone=$Number",'--jq','.number')
    }
    Write-Host "$($Plan.Title): $($Plan.Issues.Count) Issues zugeordnet (Milestone #$Number)"
}
Write-Host "https://github.com/$Repo/milestones"
