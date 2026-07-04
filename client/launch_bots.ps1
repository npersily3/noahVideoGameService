# Builds and runs the headless UDP bot swarm (bots.go + utils.go, main.go excluded
# since it declares its own func main()).
#
#   ./launch_bots.ps1 -Bots 100                         # server 127.0.0.1:34254
#   ./launch_bots.ps1 -Bots 20 -Server 127.0.0.1:7165   # point at kind/Agones hostPort
#
param(
    [int]$Bots = 10,
    [string]$Server = "127.0.0.1:34254"
)

Set-Location -Path $PSScriptRoot

$outDir = "..\build\client"
New-Item -ItemType Directory -Force -Path $outDir | Out-Null

Write-Host "Building bots..."
go build -o "$outDir\bots.exe" bots.go utils.go
if ($LASTEXITCODE -ne 0) { throw "go build failed" }

& "$outDir\bots.exe" -bots $Bots -k8s $Server
