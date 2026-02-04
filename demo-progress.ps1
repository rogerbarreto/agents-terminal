# demo-progress.ps1
# Demonstrates PTUI protocol with a progress bar updating in real-time
#
# Run this script INSIDE the Pixel Terminal to see UI components rendered

Write-Host "PTUI Progress Bar Demo"
Write-Host "======================"
Write-Host ""

# Function to send PTUI command
function Send-PTUI {
    param([string]$Json)
    # ESC ] 1337 ; PTUI=<json> BEL
    $esc = [char]0x1B
    $bel = [char]0x07
    Write-Host -NoNewline "$esc]1337;PTUI=$Json$bel"
}

# Show a progress bar
Write-Host "Starting download simulation..."
Write-Host ""

# Create the progress bar component
$progressJson = '{"type":"progress","id":"download","value":0,"max":100,"label":"Downloading file.zip"}'
Send-PTUI $progressJson

# Simulate download progress
for ($i = 1; $i -le 100; $i++) {
    Start-Sleep -Milliseconds 50
    
    # Update the progress bar value
    $updateJson = "{`"update`":`"download`",`"props`":{`"value`":$i,`"label`":`"Downloading... $i%`"}}"
    Send-PTUI $updateJson
}

Write-Host ""
Write-Host "Download complete!"

# Show success message
$successJson = '{"type":"text","id":"status","content":"✓ Download completed successfully!","color":"#48BB78","size":16}'
Send-PTUI $successJson

Start-Sleep -Seconds 2

# Clear the UI
$clearJson = '{"clear":true}'
Send-PTUI $clearJson

Write-Host ""
Write-Host "Demo finished. PTUI components cleared."
