# Launches N client instances ("gamers"), each on its own HTTP port,
# and opens a browser tab for each one.
#
#   ./launch_gamers.ps1 -N 3                              # gamers on 8080, 8081, 8082, server 127.0.0.1:34254
#   ./launch_gamers.ps1 -N 5 -BasePort 9000
#   ./launch_gamers.ps1 -N 3 -Server 127.0.0.1:7165      # point at kind/Agones hostPort
#
param(
    [int]$N = 2,
    [int]$BasePort = 8080,
    [string]$Server = "127.0.0.1:34254",
    [switch]$Debug,
    [switch]$NoBrowser
)

# Run from the client folder so index.html resolves correctly.
Set-Location -Path $PSScriptRoot

$outDir = "..\build\client"
New-Item -ItemType Directory -Force -Path $outDir | Out-Null
$exe = "$outDir\client.exe"

if ($Debug) {
    Write-Host "Building client (debug: logging sent UDP to udp_debug_<id>.txt)..."
    go build -tags debug -o $exe main.go utils.go
}
else {
    Write-Host "Building client..."
    go build -o $exe main.go utils.go
}
if ($LASTEXITCODE -ne 0) { throw "go build failed" }

$procs = @()
for ($i = 0; $i -lt $N; $i++) {
    $port = $BasePort + $i
    Write-Host "Starting gamer $($i + 1) on port $port"
    $procs += Start-Process -FilePath $exe -ArgumentList "-port", "$port", "-k8s", "$Server" -PassThru
    if (-not $NoBrowser) {
        Start-Process "http://localhost:$port"
    }
}

Write-Host ""
Write-Host "$N gamer(s) running on ports $BasePort..$($BasePort + $N - 1)."
Write-Host "Press Ctrl+C to stop, or close this window (it will kill the gamers)."

try {
    Wait-Process -Id ($procs | ForEach-Object { $_.Id })
}
finally {
    foreach ($p in $procs) {
        if (-not $p.HasExited) { Stop-Process -Id $p.Id -Force }
    }
}
