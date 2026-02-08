# AeroDB Docker Quick Start Guide

## Prerequisites Check

Docker is installed but the daemon is not running. You need to start it first.

### Start Docker Daemon

Depending on your system:

**Linux (systemd):**
```bash
sudo systemctl start docker
sudo systemctl enable docker  # Start on boot
```

**Linux (non-systemd):**
```bash
sudo service docker start
```

**Check Docker status:**
```bash
docker info
```

## Build and Run AeroDB

### Option 1: Using docker build + docker run

```bash
# 1. Build the image (takes 5-10 minutes first time)
docker build -t aerodb:latest .

# 2. Create volumes
docker volume create aerodb_data
docker volume create aerodb_logs

# 3. Run the container
docker run -d \
  --name aerodb-server \
  -p 54321:54321 \
  -v aerodb_data:/var/lib/aerodb \
  -v aerodb_logs:/var/log/aerodb \
  -v $(pwd)/docker/config.json:/etc/aerodb/config.json:ro \
  --restart unless-stopped \
  aerodb:latest

# 4. Check logs
docker logs -f aerodb-server

# 5. Check status
docker ps
```

### Option 2: Using docker-compose (if installed)

```bash
# Install docker-compose first if needed:
# sudo apt-get install docker-compose  # On Ubuntu/Debian
# sudo pip install docker-compose      # Using pip

# 1. Build and start
docker-compose up -d

# 2. View logs
docker-compose logs -f aerodb

# 3. Check status
docker-compose ps
```

## Using AeroDB

Once running, AeroDB will be available at `http://localhost:54321`

### Initialize the database
```bash
docker exec -it aerodb-server aerodb init --config /etc/aerodb/config.json
```

### Run queries
```bash
# Interactive query mode
docker exec -it aerodb-server aerodb query --config /etc/aerodb/config.json

# From your host machine (if you have aerodb CLI built)
cargo build --release
./target/release/aerodb query --config ./docker/config.json
```

### View logs  
```bash
docker exec aerodb-server aerodb logs -n 50
```

### Run migrations
```bash
docker exec aerodb-server aerodb migrate status --config /etc/aerodb/config.json
```

## Management Commands

### Stop AeroDB
```bash
docker stop aerodb-server
# or with compose:
docker-compose stop
```

### Start AeroDB
```bash
docker start aerodb-server
# or with compose:
docker-compose start
```

### Restart AeroDB
```bash
docker restart aerodb-server
# or with compose:
docker-compose restart
```

### Remove container (keeps data)
```bash
docker stop aerodb-server
docker rm aerodb-server
# or with compose:
docker-compose down
```

### Remove everything including data (WARNING!)
```bash
docker stop aerodb-server
docker rm aerodb-server
docker volume rm aerodb_data aerodb_logs
# or with compose:
docker-compose down -v
```

## Troubleshooting

### Docker daemon not running
```bash
sudo systemctl start docker
# or
sudo service docker start
```

### Permission denied
```bash
# Add your user to docker group
sudo usermod -aG docker $USER
# Log out and back in for changes to take effect
```

### Check container status
```bash
docker ps -a
docker logs aerodb-server
```

### Rebuild from scratch
```bash
docker stop aerodb-server
docker rm aerodb-server
docker rmi aerodb:latest
docker build --no-cache -t aerodb:latest .
```

### Access container shell
```bash
docker exec -it aerodb-server /bin/bash
```

## Next Steps

1. Start Docker daemon: `sudo systemctl start docker`
2. Build image: `docker build -t aerodb:latest .`
3. Run container: See "Build and Run AeroDB" above
4. Access at `http://localhost:54321`
