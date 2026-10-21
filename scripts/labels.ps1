# Creates or updates Grouper's GitHub labels. Safe to rerun: existing labels
# get the colors and descriptions below. Needs the GitHub CLI (`gh auth login`).
#   pwsh scripts/labels.ps1            (or: powershell -File scripts\labels.ps1)
param([string]$Repo = "prestonsokol1130/grouper")

$ErrorActionPreference = "Stop"

# Each group shares one color family so labels read as a set.
$labels = @(
  # Type: blue
  @{ name = "bug";      color = "1d4ed8"; desc = "Something is broken" }
  @{ name = "feature";  color = "2563eb"; desc = "New capability" }
  @{ name = "polish";   color = "3b82f6"; desc = "Looks or feels off, works fine" }
  @{ name = "chore";    color = "60a5fa"; desc = "Build, CI, deps, cleanup" }
  @{ name = "docs";     color = "93c5fd"; desc = "README, AGENTS.md, DESIGN.md, guides" }

  # Area: green
  @{ name = "area: taskbar-groups"; color = "065f46"; desc = "Group shortcuts, pins, taskbar identity" }
  @{ name = "area: flyout";         color = "047857"; desc = "Group pop-up over the taskbar" }
  @{ name = "area: launchpad";      color = "059669"; desc = "Fullscreen app grid, search, folders" }
  @{ name = "area: settings";       color = "10b981"; desc = "Settings window and options" }
  @{ name = "area: icons";          color = "34d399"; desc = "App icons, group .ico composites" }
  @{ name = "area: installer";      color = "6ee7b7"; desc = "Installer, uninstaller, updates" }

  # Priority: red
  @{ name = "P0 broken";       color = "b91c1c"; desc = "Unusable or data loss, fix now" }
  @{ name = "P1 important";    color = "ef4444"; desc = "Fix before the next release" }
  @{ name = "P2 nice-to-have"; color = "fca5a5"; desc = "When there is time" }

  # Status: purple
  @{ name = "needs-decision"; color = "6d28d9"; desc = "Waiting on Preston" }
  @{ name = "blocked";        color = "8b5cf6"; desc = "Waiting on something else" }
  @{ name = "ready";          color = "c4b5fd"; desc = "Decided and ready to build" }

  # Windows quirks: amber
  @{ name = "store-apps";       color = "b45309"; desc = "Store/MSIX apps (AppsFolder, AUMID launch)" }
  @{ name = "chrome-web-apps";  color = "d97706"; desc = "Chrome PWAs via chrome_proxy.exe" }
  @{ name = "explorer-restart"; color = "fbbf24"; desc = "Needs an Explorer restart or icon cache refresh" }

  # Other: gray
  @{ name = "from-testing";     color = "4b5563"; desc = "Found by hand while testing a build" }
  @{ name = "good-first-issue"; color = "9ca3af"; desc = "Small and self-contained" }
)

# GitHub's defaults that map onto the set: rename instead of duplicating.
$renames = @{ "enhancement" = "feature"; "documentation" = "docs" }

$existing = gh label list --repo $Repo --limit 200 --json name | ConvertFrom-Json | ForEach-Object { $_.name }

foreach ($old in $renames.Keys) {
  $new = $renames[$old]
  if ($existing -contains $old -and $existing -notcontains $new) {
    gh label edit $old --repo $Repo --name $new
    Write-Host "renamed  $old -> $new"
  }
}

foreach ($l in $labels) {
  gh label create $l.name --repo $Repo --color $l.color --description $l.desc --force | Out-Null
  Write-Host ("ok       {0,-22} #{1}" -f $l.name, $l.color)
}
