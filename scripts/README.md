# LDrive Installation Scripts

## Quick Install

### Linux/macOS
```bash
curl -fsSL https://raw.githubusercontent.com/rand0mdevel0per/ldrive/main/scripts/install.sh | bash
```

### Windows
```powershell
curl -o install.bat https://raw.githubusercontent.com/rand0mdevel0per/ldrive/main/scripts/install.bat && install.bat
```

## Manual Setup

1. Copy `config.toml.example` to `~/.config/ldrive/config.toml`
2. Edit the config file with your token and settings
3. Run: `ldrive-node storage --config ~/.config/ldrive/config.toml`

## Systemd Service (Linux)

```bash
sudo cp ldrive.service /etc/systemd/system/ldrive@.service
sudo systemctl enable ldrive@$USER
sudo systemctl start ldrive@$USER
```
