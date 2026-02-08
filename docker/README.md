# AeroDB Docker Deployment

## Quick Start

### 1. Build the Docker image

```bash
docker-compose build
```

### 2. Start AeroDB

```bash
docker-compose up -d
```

### 3. Check status

```bash
docker-compose ps
docker-compose logs -f aerodb
```

### 4. Access AeroDB

AeroDB will be available at `http://localhost:54321`

## Configuration

Edit `docker/config.json` to customize your AeroDB instance:

- `data_dir`: Data storage location (mounted as volume)
- `server.port`: Server port
- `wal.max_size`: Maximum WAL size in bytes
- `auth.jwt_secret`: **CHANGE THIS IN PRODUCTION**

## Commands

### Initialize AeroDB
```bash
docker-compose exec aerodb aerodb init --config /etc/aerodb/config.json
```

### View logs
```bash
docker-compose exec aerodb aerodb logs -n 100
```

### Run migrations
```bash
docker-compose exec aerodb aerodb migrate status --config /etc/aerodb/config.json
```

### Stop AeroDB
```bash
docker-compose down
```

### Stop and remove volumes (WARNING: deletes data)
```bash
docker-compose down -v
```

## Data Persistence

Data is persisted in Docker volumes:
- `aerodb_data`: Database data, WAL, checkpoints
- `aerodb_logs`: Log files

To backup data:
```bash
docker run --rm -v aerodb_data:/data -v $(pwd):/backup alpine tar czf /backup/aerodb-backup.tar.gz /data
```

To restore data:
```bash
docker run --rm -v aerodb_data:/data -v $(pwd):/backup alpine tar xzf /backup/aerodb-backup.tar.gz -C /
```

## Production Deployment

### Security Checklist
- [ ] Change `auth.jwt_secret` in config.json
- [ ] Use HTTPS reverse proxy (nginx, Traefik)
- [ ] Set proper file permissions
- [ ] Enable firewall rules
- [ ] Configure resource limits
- [ ] Set up monitoring

### Resource Limits

Add to docker-compose.yml:
```yaml
services:
  aerodb:
    deploy:
      resources:
        limits:
          cpus: '2'
          memory: 2G
        reservations:
          cpus: '1'
          memory: 1G
```

## Troubleshooting

### Container won't start
```bash
docker-compose logs aerodb
```

### Check health
```bash
docker-compose exec aerodb aerodb control diag diagnostics --config /etc/aerodb/config.json
```

### Reset everything
```bash
docker-compose down -v
docker-compose build --no-cache
docker-compose up -d
```
