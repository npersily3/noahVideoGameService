# Launches N client instances ("gamers"), each on its own HTTP port,
# and opens a browser tab for each one.
#
#   ./launch_gamers.ps1 -N 3            # gamers on 8080, 8081, 8082
#   ./launch_gamers.ps1 -N 5 -BasePort 9000
#
param(
    [int]$N = 2,
    [int]$BasePort = 8080,
    [switch]$NoBrowser
)

# Run from the client folder so index.html resolves correctly.
Set-Location -Path $PSScriptRoot

Write-Host "Building client..."
go build -o client.exe .
if ($LASTEXITCODE -ne 0) { throw "go build failed" }

$procs = @()
for ($i = 0; $i -lt $N; $i++) {
    $port = $BasePort + $i
    Write-Host "Starting gamer $($i + 1) on port $port"
    $procs += Start-Process -FilePath ".\client.exe" -ArgumentList "-port", "$port" -PassThru
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
