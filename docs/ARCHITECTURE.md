# Architecture

OpenFoot Manager is a desktop football management simulation built with **Tauri** (Rust backend) and **React** (TypeScript frontend). This document describes the project structure, key architectural decisions, and how the pieces fit together.

---

## Technology Stack

| Layer | Technology | Purpose |
|-------|-----------|---------|
| **Desktop shell** | Tauri v2 | Native window, IPC, file system access |
| **Backend** | Rust | Game logic, simulation, persistence |
| **Frontend** | React + TypeScript | UI rendering, user interaction |
| **Styling** | Tailwind CSS | Utility-first CSS framework |
| **State (frontend)** | Zustand | Lightweight stores for game and settings |
| **i18n** | i18next + react-i18next | Internationalization (7 locales) |
| **Build** | Vite | Frontend bundler and dev server |

---

## Project Structure