# Quick Start Guide

This guide will help you get AeroDB up and running on your local machine in minutes.

## Prerequisites

Before you begin, ensure you have the following installed:

*   **Rust** (1.70 or later): `rustc --version`
*   **Node.js** (18 or later): `node --version`
*   **Git**: `git --version`

## Step 1: Clone the Repository

```bash
git clone https://github.com/eshanized/AeroDB.git
cd AeroDB
```

## Step 2: Build the Backend

Compile the Rust backend in release mode for optimal performance:

```bash
cargo build --release
```

!!! tip "Compilation Time"
    The first build might take a few minutes as it compiles all dependencies. Subsequent builds will be much faster.

## Step 3: Install Frontend Dependencies

Navigate to the dashboard directory and install the necessary packages:

```bash
cd dashboard
npm install
cd ..
```

## Step 4: Run the Server

Start the AeroDB backend server:

```bash
# Run from the root directory
cargo run --release -- serve
```

You should see output indicating the server is running on `http://localhost:54321`.

## Step 5: Start the Dashboard

In a new terminal window, start the frontend development server:

```bash
cd dashboard
npm run dev
```

Open your browser and navigate to `http://localhost:5173`.

## Step 6: First-Run Setup

Since this is a fresh installation, you will be redirected to the **Setup Wizard**.

1.  **Welcome**: Click "Start Setup".
2.  **Storage**: Choose where AeroDB should store its data. The defaults (`./data`, `./wal`, etc.) are fine for development.
3.  **Authentication**: Set your JWT expiration preferences (e.g., 24 hours).
4.  **Admin User**: Create your super-admin account. **Remember these credentials!**
5.  **Review**: Confirm your settings.
6.  **Complete**: Click "Go to Dashboard".

## Next Steps

Congratulations! You now have a running instance of AeroDB.

*   [Explore the Core Principles](../CORE_VISION.md)
*   [Learn about the Architecture](../CORE_SCOPE.md)
*   [Read the API Specification](../CORE_API_SPEC.md)

!!! warning "Production Deployment"
    For production environments, ensure you configure **HTTPS** and use a process manager like `systemd` or `supervisord`.
