# HF Space Deployment Helper
# This script automates the upload and push process.

# Set your token here or as an environment variable
$TOKEN = $env:HF_TOKEN
$REPO = "trioskosmos/rabukasim"
$HF_CLI = "C:\Users\trios\.local\bin\hf.exe"

Write-Host "Installing HF CLI..." -ForegroundColor Cyan
powershell -ExecutionPolicy ByPass -c "irm https://hf.co/cli/install.ps1 | iex"

# CRITICAL: We use [LOCAL_PATH] [PATH_IN_REPO] to prevent files from landing in the root folder.
Write-Host "Uploading core components..." -ForegroundColor Cyan
& $HF_CLI upload $REPO engine/ engine/ --repo-type=space --token $TOKEN
& $HF_CLI upload $REPO cards/ cards/ --repo-type=space --token $TOKEN
& $HF_CLI upload $REPO Dockerfile Dockerfile --repo-type=space --token $TOKEN
& $HF_CLI upload $REPO README.md README.md --repo-type=space --token $TOKEN
& $HF_CLI upload $REPO .gitattributes .gitattributes --repo-type=space --token $TOKEN

Write-Host "Uploading images in batches to avoid CLI timeouts..." -ForegroundColor Cyan
# We batch the images to prevent the 'Checking' phase from exceeding the environment's timeout.
# The hf CLI automatically skips files that already exist with the same hash.
$images = Get-ChildItem "web_ui/img/cards_webp/*.webp"
$batchSize = 200
for ($i = 0; $i -lt $images.Count; $i += $batchSize) {
    $batch = $images[$i..($i + $batchSize - 1)]
    Write-Host "Uploading batch $($i + 1) to $($i + $batchSize)..." -ForegroundColor Yellow
    foreach ($file in $batch) {
        if ($file -ne $null) {
            & $HF_CLI upload $REPO $file.FullName "web_ui/img/cards_webp/$($file.Name)" --repo-type=space --token $TOKEN
        }
    }
}

Write-Host "Uploading remaining web_ui files..." -ForegroundColor Cyan
& $HF_CLI upload $REPO web_ui/ web_ui/ --repo-type=space --token $TOKEN

Write-Host "Pushing to Git main branch..." -ForegroundColor Cyan
git branch -m master main 2>$null
git push space main --force

Write-Host "Deployment Complete!" -ForegroundColor Green
