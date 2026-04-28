"""History management for chat conversations and user feedback."""

import json
import os
import hashlib
import uuid
from datetime import datetime
from pathlib import Path
from typing import List, Dict, Any, Optional


class HistoryManager:
    """Manages conversation history and user feedback persistence."""

    HISTORY_FILE = "data/chat_history.json"
    FEEDBACK_FILE = "data/feedback.json"

    def __init__(self):
        Path(self.HISTORY_FILE).parent.mkdir(parents=True, exist_ok=True)
        self._ensure_files()

    def _ensure_files(self):
        """Ensure history and feedback files exist."""
        if not Path(self.HISTORY_FILE).exists():
            self._save_history({"conversations": {}})
        if not Path(self.FEEDBACK_FILE).exists():
            self._save_feedback({"feedback": {}})

    def _load_history(self) -> dict:
        """Load chat history from file."""
        try:
            with open(self.HISTORY_FILE, "r", encoding="utf-8") as f:
                return json.load(f)
        except (json.JSONDecodeError, FileNotFoundError):
            return {"conversations": {}}

    def _save_history(self, data: dict):
        """Save chat history to file."""
        with open(self.HISTORY_FILE, "w", encoding="utf-8") as f:
            json.dump(data, f, ensure_ascii=False, indent=2)

    def _load_feedback(self) -> dict:
        """Load feedback data from file."""
        try:
            with open(self.FEEDBACK_FILE, "r", encoding="utf-8") as f:
                return json.load(f)
        except (json.JSONDecodeError, FileNotFoundError):
            return {"feedback": {}}

    def _save_feedback(self, data: dict):
        """Save feedback data to file."""
        with open(self.FEEDBACK_FILE, "w", encoding="utf-8") as f:
            json.dump(data, f, ensure_ascii=False, indent=2)

    def _generate_id(self) -> str:
        """Generate a unique conversation ID."""
        return uuid.uuid4().hex[:12]

    def _hash_content(self, question: str, answer: str) -> str:
        """Generate a hash for deduplicating feedback."""
        content = f"{question}|{answer}"
        return hashlib.sha256(content.encode()).hexdigest()[:16]

    # ─────────────────────────────────────────────────────────────────
    # Conversation History
    # ─────────────────────────────────────────────────────────────────

    def list_conversations(self) -> List[dict]:
        """List all conversations sorted by most recent.

        Returns:
            List of dicts with id, title, preview, timestamp, message_count.
        """
        data = self._load_history()
        conversations = []
        for conv_id, conv_data in data.get("conversations", {}).items():
            messages = conv_data.get("messages", [])
            preview = ""
            if messages:
                first_user = next(
                    (m["content"][:50] for m in messages if m["role"] == "user"),
                    ""
                )
                preview = first_user + ("..." if len(first_user) == 50 else "")

            conversations.append({
                "id": conv_id,
                "title": conv_data.get("title", "Untitled"),
                "preview": preview or "Empty conversation",
                "created_at": conv_data.get("created_at", ""),
                "updated_at": conv_data.get("updated_at", ""),
                "message_count": len(messages)
            })

        conversations.sort(
            key=lambda x: x.get("updated_at", ""),
            reverse=True
        )
        return conversations

    def create_conversation(self) -> str:
        """Create a new empty conversation.

        Returns:
            The new conversation ID.
        """
        data = self._load_history()
        conv_id = self._generate_id()
        now = datetime.now().isoformat()
        data["conversations"][conv_id] = {
            "title": "New chat",
            "created_at": now,
            "updated_at": now,
            "messages": []
        }
        self._save_history(data)
        return conv_id

    def get_conversation(self, conv_id: str) -> List[dict]:
        """Get all messages in a conversation.

        Args:
            conv_id: The conversation ID.

        Returns:
            List of message dicts with role, content, timestamp, and optional sources.
        """
        data = self._load_history()
        conv = data.get("conversations", {}).get(conv_id)
        if not conv:
            return []
        return conv.get("messages", [])

    def update_conversation_title(self, conv_id: str, title: str):
        """Update the title of a conversation.

        Args:
            conv_id: The conversation ID.
            title: New title.
        """
        data = self._load_history()
        if conv_id in data["conversations"]:
            data["conversations"][conv_id]["title"] = title
            self._save_history(data)

    def append_message(
        self,
        conv_id: str,
        role: str,
        content: str,
        sources: Optional[List[dict]] = None
    ):
        """Append a message to a conversation.

        Args:
            conv_id: The conversation ID.
            role: 'user' or 'assistant'.
            content: Message text.
            sources: Optional list of source dicts (for assistant messages).
        """
        data = self._load_history()
        if conv_id not in data["conversations"]:
            return

        now = datetime.now().isoformat()
        message = {
            "role": role,
            "content": content,
            "timestamp": now
        }
        if role == "assistant" and sources:
            message["sources"] = sources

        data["conversations"][conv_id]["messages"].append(message)
        data["conversations"][conv_id]["updated_at"] = now

        # Auto-generate title from first user message
        conv = data["conversations"][conv_id]
        if conv["title"] == "New chat" and role == "user":
            conv["title"] = content[:40] + ("..." if len(content) > 40 else "")

        self._save_history(data)

    def delete_conversation(self, conv_id: str):
        """Delete a conversation.

        Args:
            conv_id: The conversation ID.
        """
        data = self._load_history()
        if conv_id in data["conversations"]:
            del data["conversations"][conv_id]
            self._save_history(data)

    # ─────────────────────────────────────────────────────────────────
    # User Feedback
    # ─────────────────────────────────────────────────────────────────

    def save_feedback(
        self,
        question: str,
        answer: str,
        rating: str,
        sources: Optional[List[dict]] = None
    ) -> bool:
        """Save user feedback for a query-response pair.

        Args:
            question: The user's question.
            answer: The assistant's answer.
            rating: 'thumbs_up' or 'thumbs_down'.
            sources: Optional list of cited sources.

        Returns:
            True if saved, False if duplicate exists (overwrites).
        """
        feedback_key = self._hash_content(question, answer)
        data = self._load_feedback()
        data["feedback"][feedback_key] = {
            "question": question,
            "answer": answer,
            "rating": rating,
            "timestamp": datetime.now().isoformat(),
            "sources": sources or []
        }
        self._save_feedback(data)
        return True

    def get_feedback_stats(self) -> dict:
        """Get aggregated feedback statistics.

        Returns:
            Dict with thumbs_up and thumbs_down counts.
        """
        data = self._load_feedback()
        feedback_list = data.get("feedback", {}).values()
        thumbs_up = sum(1 for f in feedback_list if f.get("rating") == "thumbs_up")
        thumbs_down = sum(1 for f in feedback_list if f.get("rating") == "thumbs_down")
        return {"thumbs_up": thumbs_up, "thumbs_down": thumbs_down}

    def get_all_feedback(self) -> List[dict]:
        """Get all feedback entries.

        Returns:
            List of feedback dicts sorted by timestamp descending.
        """
        data = self._load_feedback()
        feedback_list = list(data.get("feedback", {}).values())
        feedback_list.sort(key=lambda x: x.get("timestamp", ""), reverse=True)
        return feedback_list
