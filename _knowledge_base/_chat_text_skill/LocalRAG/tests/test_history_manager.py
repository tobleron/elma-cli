"""Tests for history_manager module."""

import os
import pytest
import tempfile
import shutil
import json

from src.history_manager import HistoryManager


class TestHistoryManagerBasics:
    """Tests for HistoryManager conversation operations."""

    def setup_method(self):
        """Set up test fixtures with isolated temp directories."""
        self.temp_dir = tempfile.mkdtemp()
        self._orig_history = HistoryManager.HISTORY_FILE
        self._orig_feedback = HistoryManager.FEEDBACK_FILE
        HistoryManager.HISTORY_FILE = os.path.join(self.temp_dir, "chat_history.json")
        HistoryManager.FEEDBACK_FILE = os.path.join(self.temp_dir, "feedback.json")
        self.hm = HistoryManager()

    def teardown_method(self):
        """Clean up temp files."""
        try:
            shutil.rmtree(self.temp_dir)
        except:
            pass
        HistoryManager.HISTORY_FILE = self._orig_history
        HistoryManager.FEEDBACK_FILE = self._orig_feedback

    # ─────────────────────────────────────────────────────────────────
    # Conversation tests
    # ─────────────────────────────────────────────────────────────────

    def test_list_conversations_empty(self):
        """New manager returns empty list."""
        assert self.hm.list_conversations() == []

    def test_create_conversation_returns_id(self):
        """create_conversation returns a non-empty string ID."""
        conv_id = self.hm.create_conversation()
        assert isinstance(conv_id, str)
        assert len(conv_id) > 0

    def test_create_conversation_appears_in_list(self):
        """After creating a conversation, it appears in list_conversations."""
        conv_id = self.hm.create_conversation()
        convs = self.hm.list_conversations()
        assert len(convs) == 1
        assert convs[0]["id"] == conv_id

    def test_get_conversation_empty(self):
        """get_conversation on non-existent ID returns empty list."""
        assert self.hm.get_conversation("nonexistent") == []

    def test_get_conversation_after_append(self):
        """get_conversation returns messages appended to it."""
        conv_id = self.hm.create_conversation()
        self.hm.append_message(conv_id, "user", "Hello")
        self.hm.append_message(conv_id, "assistant", "Hi there")
        msgs = self.hm.get_conversation(conv_id)
        assert len(msgs) == 2
        assert msgs[0]["role"] == "user"
        assert msgs[0]["content"] == "Hello"
        assert msgs[1]["role"] == "assistant"
        assert msgs[1]["content"] == "Hi there"

    def test_append_message_auto_title(self):
        """First user message becomes conversation title."""
        conv_id = self.hm.create_conversation()
        assert self.hm.list_conversations()[0]["title"] == "New chat"
        self.hm.append_message(conv_id, "user", "What is Python?")
        convs = self.hm.list_conversations()
        assert convs[0]["title"] == "What is Python?"

    def test_update_conversation_title(self):
        """update_conversation_title changes the title."""
        conv_id = self.hm.create_conversation()
        self.hm.update_conversation_title(conv_id, "My Custom Title")
        convs = self.hm.list_conversations()
        assert convs[0]["title"] == "My Custom Title"

    def test_delete_conversation(self):
        """delete_conversation removes it from the list."""
        conv_id = self.hm.create_conversation()
        self.hm.append_message(conv_id, "user", "Test")
        assert len(self.hm.list_conversations()) == 1
        self.hm.delete_conversation(conv_id)
        assert len(self.hm.list_conversations()) == 0

    def test_append_message_stores_sources(self):
        """assistant message with sources is stored correctly."""
        conv_id = self.hm.create_conversation()
        sources = [{"source": "doc.txt", "text": "some content"}]
        self.hm.append_message(conv_id, "assistant", "Answer", sources=sources)
        msgs = self.hm.get_conversation(conv_id)
        assert msgs[0]["sources"] == sources

    # ─────────────────────────────────────────────────────────────────
    # Feedback tests
    # ─────────────────────────────────────────────────────────────────

    def test_feedback_stats_empty(self):
        """Empty feedback returns zeros."""
        stats = self.hm.get_feedback_stats()
        assert stats["thumbs_up"] == 0
        assert stats["thumbs_down"] == 0

    def test_save_feedback_thumbs_up(self):
        """save_feedback with thumbs_up increments counter."""
        self.hm.save_feedback("Q1", "A1", "thumbs_up")
        stats = self.hm.get_feedback_stats()
        assert stats["thumbs_up"] == 1
        assert stats["thumbs_down"] == 0

    def test_save_feedback_thumbs_down(self):
        """save_feedback with thumbs_down increments counter."""
        self.hm.save_feedback("Q1", "A1", "thumbs_down")
        stats = self.hm.get_feedback_stats()
        assert stats["thumbs_up"] == 0
        assert stats["thumbs_down"] == 1

    def test_save_feedback_multiple(self):
        """Multiple feedback entries counted correctly."""
        self.hm.save_feedback("Q1", "A1", "thumbs_up")
        self.hm.save_feedback("Q2", "A2", "thumbs_up")
        self.hm.save_feedback("Q3", "A3", "thumbs_down")
        stats = self.hm.get_feedback_stats()
        assert stats["thumbs_up"] == 2
        assert stats["thumbs_down"] == 1

    def test_save_feedback_overwrites(self):
        """Same question+answer overwrites previous feedback."""
        self.hm.save_feedback("Q1", "A1", "thumbs_up")
        self.hm.save_feedback("Q1", "A1", "thumbs_down")
        stats = self.hm.get_feedback_stats()
        assert stats["thumbs_up"] == 0
        assert stats["thumbs_down"] == 1

    def test_get_all_feedback_sorted(self):
        """get_all_feedback returns entries sorted by timestamp desc."""
        self.hm.save_feedback("Q1", "A1", "thumbs_up")
        self.hm.save_feedback("Q2", "A2", "thumbs_down")
        all_fb = self.hm.get_all_feedback()
        assert len(all_fb) == 2
        # Most recent first
        assert all_fb[0]["question"] == "Q2"

    def test_feedback_with_sources(self):
        """Feedback stores sources list."""
        sources = [{"source": "doc.txt", "text": "content"}]
        self.hm.save_feedback("Q", "A", "thumbs_up", sources=sources)
        all_fb = self.hm.get_all_feedback()
        assert all_fb[0]["sources"] == sources
