# Slate: Local Memory for Coding Agents

Slate is a structured, semantic memory system designed to replace flat-file memory solutions (like `memory.md` or `logs/`) for locally running coding agents. Instead of appending text to a file that grows indefinitely and becomes hard to search, Slate provides a sophisticated cognitive architecture with:

- **Short-term Memory (Context):** A high-performance buffer for immediate thoughts and task state that naturally decays.
- **Long-term Memory (History):** A permanent, vector-indexed store for experiences, accessible via semantic search.
- **Procedural Memory (Skills):** A secure runtime for executing deterministic logic.

This guide explains how to set up Slate locally and integrate it into your Python-based coding agent.

## Prerequisites

- **Docker & Docker Compose**: For running the Slate Server and Postgres database.
- **OpenAI API Key**: Required for generating embeddings and summarizing memory.

## Installation & Running

Slate runs as a local service alongside your agent.

1.  **Clone the Repository**

    ```bash
    git clone https://github.com/your-org/slate.git
    cd slate
    ```

2.  **Configure Environment**
    Create a `.env` file in the root of the repository with your OpenAI API key:

    ```bash
    OPENAI_API_KEY=sk-proj-...
    ```

3.  **Start the Server**
    Run the following command to start the Postgres database (with `pgvector`) and the Slate Server:

    ```bash
    docker compose up --build -d
    ```

    - **API Server:** `http://localhost:3001`
    - **Database:** `localhost:5432`

    You can verify the server is running by visiting `http://localhost:3001/health` (if available) or checking the logs.

4.  **Optional: Dashboard**
    To visualize the memory state in real-time:
    ```bash
    make dashboard
    ```
    This launches a Vue.js interface for inspecting Context and History.

## Memory Types & Usage Options

Slate provides three distinct types of memory, each designed for a specific cognitive function. Understanding when to use each is key to building a robust agent.

### 1. Context (Short-Term / Working Memory)

**Best for:** Immediate thoughts, scratchpad data, current tool outputs, and maintaining the "thread" of conversation.

- **Mechanism:** Context is transient. Items are stored with a `relevance_score` (importance) and `decay_rate` (how fast they fade).
- **Lifecycle:**
  1.  **Focus:** You push an item to context.
  2.  **Decay:** Every few minutes/ticks, the item's score decreases.
  3.  **Pruning:** Low-score items are removed or consolidated to History.

**Usage Options:**

- **Standard Log:** Keep track of recent actions.

  ```python
  client.focus("Read file main.py")
  ```

- **High Importance:** Force an item to stay in memory longer (e.g., user instructions).

  ```python
  # relevance=1.5 (default 1.0) makes it start stronger
  # decay=0.01 (default 0.1) makes it fade slower
  client.focus("User Goal: Fix the login bug", relevance=1.5, decay=0.01)
  ```

- **Transient Thought:** Store a quick intermediate result that shouldn't last long.
  ```python
  # High decay means it will be gone quickly
  client.focus("Temp calc result: 42", decay=0.5)
  ```

### 2. History (Long-Term / Episodic Memory)

**Best for:** Completed tasks, learned facts, significant milestones, and "lessons learned" that should persist across sessions.

- **Mechanism:** Stored permanently in a vector database (`pgvector`).
- **Structure:** Unlike flat logs, History is structured into "Experiences" with 4 key fields:
  - `input`: The trigger or situation (e.g., "Error: Connection Refused").
  - `action`: What the agent did (e.g., "Updated connection string").
  - `outcome`: The result (e.g., "Success, database connected").
  - `reasoning`: Why it worked (e.g., "Old port was 5432, new is 5433").

**Usage Options:**

- **Commit Experience:** Save a structured lesson.

  ```python
  client.commit(
      input="Request to genericize the API client",
      action="Refactor",
      outcome="Created AbstractClient class",
      reasoning="Allows swapping backends easily"
  )
  ```

- **Semantic Search (Reminisce):** Retrieve past experiences.
  ```python
  # Returns traces that match the *meaning* of the query, not just keywords.
  memories = client.reminisce("How did we fix the DB connection?")
  for m in memories.traces:
      print(f"Action: {m.action}, Outcome: {m.outcome}")
  ```

### 3. Procedural Memory (Skills)

**Best for:** Reusable code snippets, validated tools, or complex logic that the agent has "mastered".

- **Mechanism:** You "teach" the system a skill (code), and it can be "triggered" later.
- **Usage:** Instead of rewriting a complex regex or data validation script every time, the agent can store it as a skill and just call it.

  ```python
  # Trigger a pre-learned skill
  result = client.trigger("validate_email", {"email": "test@example.com"})
  ```

---

## Choosing the Right Memory Type

| Use Case                                  | Recommended Type        | Why?                                                 |
| :---------------------------------------- | :---------------------- | :--------------------------------------------------- |
| **"I just ran a command and got output"** | **Context**             | It's relevant now but won't be in 2 days.            |
| **"The user told me to always use tabs"** | **Context (Low Decay)** | Needs to stay active for the whole session.          |
| **"I learned that API X needs header Y"** | **History**             | This is a permanent fact useful for future sessions. |
| **"I finished the task successfully"**    | **History**             | Captures the final outcome for reporting/recall.     |
| **"I wrote a reusable utility function"** | **Skill**               | Execute it deterministically later.                  |

## Integration Guide (Python)

To connect your agent to Slate, use the Python client provided in `clients/python`.

### 1. Install the Client

You can install the client from the local repository:

```bash
pip install -e ./clients/python
```

### 2. Initialize the Client

Initialize the client at the start of your agent's session.

```python
from slate_client import CortexClient

# Connect to local instance (default port 3001)
# 'run_id' helps group memories by session or task
client = CortexClient(
    address="localhost:3001",
    token="dev_secret",
    run_id="agent-session-123"
)
```

### 3. Workflow: Start of Task (Read Memory)

Instead of reading a `memory.md` file, search for relevant past experiences.

```python
def on_task_start(task_description):
    print(f"Starting task: {task_description}")

    # Semantic search for relevant history
    memories = client.reminisce(task_description, limit=5)

    if memories.traces:
        print("Recall from previous sessions:")
        for m in memories.traces:
            print(f" - {m.input} -> {m.outcome}")

    # Add task to working memory
    client.focus(f"Starting task: {task_description}")
```

### 4. Workflow: During Task (Working Memory)

Capture thoughts, tool executions, and intermediate results.

```python
def on_tool_execution(tool_name, args, result):
    # Log activity to Short-Term Context
    # This keeps the agent "aware" of what it just did
    client.focus(f"Ran {tool_name} with {args}. Result: {result}")

def on_thought(thought_text):
    client.focus(f"Thinking: {thought_text}")
```

### 5. Workflow: End of Task (Write Memory)

When a task is completed or a significant milestone is reached, explicitly commit it to Long-Term History.

```python
def on_task_completion(task, result, summary):
    # Commit the experience to permanent storage
    client.commit(
        input=task,
        outcome=result,
        action="Task Completion",
        reasoning=summary
    )
    print("Experience saved to Slate.")
```

## Direct API Access

If your agent is written in a language other than Python, you can interact with Slate directly via its REST API.

**Base URL:** `http://localhost:3001`

### 1. Working Memory (Context)

**Add item to Context (Focus):**

- **Endpoint:** `POST /v1/context`
- **Body:**
  ```json
  {
    "project_id": "your-project-uuid",
    "content": "Running database migration...",
    "relevance_score": 1.0,
    "decay_rate": 0.1
  }
  ```

**Read Context (Drift):**

- **Endpoint:** `GET /v1/context?project_id=your-project-uuid&limit=10`
- **Response:** JSON array of active context nodes.

### 2. Long-Term Memory (History)

**Save Experience (Commit):**

- **Endpoint:** `POST /v1/history`
- **Body:**
  ```json
  {
    "project_id": "your-project-uuid",
    "content": "Database migration failed due to lock timeout",
    "metadata": {
      "action": "Run Migration",
      "outcome": "Failure: Lock Timeout",
      "reasoning": "A long-running transaction was blocking the migration."
    }
  }
  ```

**Search History (Reminisce):**

- **Endpoint:** `POST /v1/history/search`
- **Body:**
  ```json
  {
    "project_id": "your-project-uuid",
    "query": "How to fix lock timeout errors?",
    "limit": 5
  }
  ```

### 3. Skills

**Execute Skill:**

- **Endpoint:** `POST /v1/skills/{skill_name}/run`
- **Body:**
  ```json
  {
    "project_id": "your-project-uuid",
    "input": { "arg1": "value1" }
  }
  ```

## Migration Strategy

If your agent currently uses a Markdown file for memory, here is how to migrate:

| Action             | File-Based Approach (`memory.md`)     | Slate Approach                             |
| :----------------- | :------------------------------------ | :----------------------------------------- |
| **Read Context**   | `cat memory.md`                       | `client.reminisce("current query")`        |
| **Log Activity**   | `echo "Ran test" >> memory.md`        | `client.focus("Ran test")`                 |
| **Save Knowledge** | `echo "Fix: usage of X" >> memory.md` | `client.commit(outcome="Fix: usage of X")` |
| **Search**         | `grep "error" memory.md`              | `client.reminisce("error")` (Semantic)     |

### Benefits of Migration

1.  **Semantic Search:** Find "login issues" even if you search for "authentication bugs".
2.  **Automatic Management:** No need to truncate files; Slate handles decay and consolidation.
3.  **Structured Data:** Store metadata (timestamps, outcomes) alongside text.
