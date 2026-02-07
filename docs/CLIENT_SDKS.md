# Client SDKs

AeroDB provides official client libraries for multiple languages:

## JavaScript/TypeScript

**Repository**: [AeroDBs/aerodb-js](https://github.com/AeroDBs/aerodb-js)

```bash
npm install @aerodb/client
```

```typescript
import { AeroDBClient } from '@aerodb/client';

const client = new AeroDBClient({
  url: 'https://your-project.aerodb.com',
  key: 'your-api-key',
});

const { data } = await client.from('users').select('*').execute();
```

[View Documentation →](https://github.com/AeroDBs/aerodb-js)

---

## Python

**Repository**: [AeroDBs/aerodb-py](https://github.com/AeroDBs/aerodb-py)

```bash
pip install aerodb-py
```

```python
from aerodb import AeroDBClient

async with AeroDBClient(url="https://your-project.aerodb.com", key="your-key") as client:
    result = await client.from_("users").select("*").execute()
```

[View Documentation →](https://github.com/AeroDBs/aerodb-py)

---

## Features

All SDKs provide:
- ✅ **Type-safe** - Full TypeScript/type hints support
- ✅ **Result pattern** - No exceptions, explicit `{ data, error }` returns
- ✅ **Auth** - User signup, signin, session management
- ✅ **Database** - Fluent query builder for PostgREST
- ✅ **Realtime** - WebSocket subscriptions with auto-reconnect
- ✅ **Storage** - File upload/download/delete
- ✅ **Functions** - Serverless function invocation
