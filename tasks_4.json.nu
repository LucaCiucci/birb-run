#!/usr/bin/env nu

{
  tasks: {
    hello: {
      steps: [
        $"echo hello from ($env.FILE_PWD)"
      ]
    }
  }
} | to json