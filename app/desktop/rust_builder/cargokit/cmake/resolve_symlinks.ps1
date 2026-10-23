# Resolve a Flutter plugin symlink or junction to its physical path.
# Windows PowerShell 5.1 has no LinkTarget, and Write-Host is not stdout
# when CMake captures the process. Print exactly one path on the success stream.
$ErrorActionPreference = 'Stop'

function Resolve-CbdeskPath {
    param([Parameter(Mandatory = $true)][string] $InputPath)

    $full = [System.IO.Path]::GetFullPath($InputPath)
    $root = [System.IO.Path]::GetPathRoot($full)
    $rest = $full.Substring($root.Length)
    $current = $root.TrimEnd('\')
    if ($current.EndsWith(':')) {
        $current = $current + '\'
    }

    foreach ($part in $rest.Split([char[]]@('\'), [System.StringSplitOptions]::RemoveEmptyEntries)) {
        if ($current.EndsWith('\')) {
            $current = $current + $part
        } else {
            $current = Join-Path $current $part
        }
        $item = Get-Item -LiteralPath $current -Force
        $guard = 0
        while (($item.Attributes -band [System.IO.FileAttributes]::ReparsePoint) -and ($guard -lt 8)) {
            $guard++
            $target = $null
            if ($item.PSObject.Properties.Name -contains 'LinkTarget' -and $item.LinkTarget) {
                $target = [string]$item.LinkTarget
            } elseif ($item.PSObject.Properties.Name -contains 'Target' -and $item.Target) {
                $raw = $item.Target
                if ($raw -is [System.Array]) {
                    $target = [string]$raw[0]
                } else {
                    $target = [string]$raw
                }
            }
            if (-not $target) {
                break
            }
            if (-not [System.IO.Path]::IsPathRooted($target)) {
                $parent = Split-Path -Parent $item.FullName
                $target = Join-Path $parent $target
            }
            $item = Get-Item -LiteralPath $target -Force
        }
        $current = $item.FullName
    }
    return $current
}

Write-Output (Resolve-CbdeskPath -InputPath $args[0])
