import { User, Sparkles, Copy, Check, Wrench, Loader2, CheckCircle2, XCircle, ChevronDown, ChevronUp, Coins } from 'lucide-react'
import { useState } from 'react'
import ReactMarkdown from 'react-markdown'
import { Prism as SyntaxHighlighter } from 'react-syntax-highlighter'
import { oneDark } from 'react-syntax-highlighter/dist/esm/styles/prism'
import { cn } from '../lib/utils'
import type { Message, ToolInvocation } from '../lib/types'

interface MessageBubbleProps {
  message: Message
  isStreaming?: boolean
}

export function MessageBubble({ message, isStreaming }: MessageBubbleProps) {
  const isUser = message.role === 'user'

  return (
    <div
      className={cn(
        "flex gap-4",
        isUser ? "flex-row-reverse" : "flex-row"
      )}
    >
      {/* Avatar */}
      <div
        className={cn(
          "flex-shrink-0 h-8 w-8 rounded-full flex items-center justify-center",
          isUser
            ? "bg-primary-500"
            : "bg-gradient-to-br from-aura-400 to-primary-500"
        )}
      >
        {isUser ? (
          <User className="h-4 w-4 text-white" />
        ) : (
          <Sparkles className="h-4 w-4 text-white" />
        )}
      </div>

      {/* Message content */}
      <div
        className={cn(
          "flex-1 min-w-0 max-w-[85%]",
          isUser && "flex flex-col items-end"
        )}
      >
        <div
          className={cn(
            "rounded-2xl px-4 py-3",
            isUser
              ? "bg-primary-500 text-white rounded-tr-md"
              : "bg-secondary rounded-tl-md"
          )}
        >
          {isUser ? (
            <p className="whitespace-pre-wrap break-words">{message.content}</p>
          ) : (
            <div className="space-y-3">
              {/* Tool Invocations */}
              {message.toolInvocations && message.toolInvocations.length > 0 && (
                <ToolInvocations invocations={message.toolInvocations} />
              )}

              {/* Message Content */}
              {message.content && (
                <div className="markdown-content prose prose-sm dark:prose-invert max-w-none">
                  <ReactMarkdown
                    components={{
                      code({ className, children, ...props }) {
                        const inline = !className
                        const match = /language-(\w+)/.exec(className || '')
                        const language = match ? match[1] : ''

                        if (inline) {
                          return (
                            <code
                              className="bg-gray-800 px-1.5 py-0.5 rounded text-sm font-mono"
                              {...props}
                            >
                              {children}
                            </code>
                          )
                        }

                        return (
                          <CodeBlock language={language}>
                            {String(children).replace(/\n$/, '')}
                          </CodeBlock>
                        )
                      },
                    }}
                  >
                    {message.content}
                  </ReactMarkdown>
                  {isStreaming && <TypingIndicator />}
                </div>
              )}

              {/* Show typing indicator when streaming with no content yet */}
              {isStreaming && !message.content && <TypingIndicator />}
            </div>
          )}
        </div>

        {/* Usage info (tokens and cost) */}
        {!isUser && message.usage && !isStreaming && (
          <UsageDisplay usage={message.usage} />
        )}
      </div>
    </div>
  )
}

interface CodeBlockProps {
  language: string
  children: string
}

function CodeBlock({ language, children }: CodeBlockProps) {
  const [copied, setCopied] = useState(false)

  const handleCopy = async () => {
    await navigator.clipboard.writeText(children)
    setCopied(true)
    setTimeout(() => setCopied(false), 2000)
  }

  return (
    <div className="relative group my-4 -mx-4 sm:mx-0 sm:rounded-lg overflow-hidden">
      {/* Header */}
      <div className="flex items-center justify-between px-4 py-2 bg-gray-800 border-b border-gray-700">
        <span className="text-xs text-gray-400 font-mono">
          {language || 'code'}
        </span>
        <button
          onClick={handleCopy}
          className="flex items-center gap-1 text-xs text-gray-400 hover:text-gray-200 transition-colors"
        >
          {copied ? (
            <>
              <Check className="h-3.5 w-3.5" />
              Copied!
            </>
          ) : (
            <>
              <Copy className="h-3.5 w-3.5" />
              Copy
            </>
          )}
        </button>
      </div>

      {/* Code */}
      <SyntaxHighlighter
        style={oneDark}
        language={language || 'text'}
        PreTag="div"
        customStyle={{
          margin: 0,
          padding: '1rem',
          background: '#1e1e2e',
          fontSize: '0.875rem',
        }}
      >
        {children}
      </SyntaxHighlighter>
    </div>
  )
}

function TypingIndicator() {
  return (
    <span className="inline-flex items-center gap-1 ml-1">
      <span className="typing-dot h-1.5 w-1.5 rounded-full bg-current opacity-60" />
      <span className="typing-dot h-1.5 w-1.5 rounded-full bg-current opacity-60" />
      <span className="typing-dot h-1.5 w-1.5 rounded-full bg-current opacity-60" />
    </span>
  )
}

interface ToolInvocationsProps {
  invocations: ToolInvocation[]
}

function ToolInvocations({ invocations }: ToolInvocationsProps) {
  const [expandedTools, setExpandedTools] = useState<Set<string>>(new Set())

  const toggleTool = (id: string) => {
    setExpandedTools((prev) => {
      const next = new Set(prev)
      if (next.has(id)) {
        next.delete(id)
      } else {
        next.add(id)
      }
      return next
    })
  }

  return (
    <div className="space-y-2">
      {invocations.map((invocation) => (
        <div
          key={invocation.toolCallId}
          className="rounded-lg border border-border/50 bg-background/50 overflow-hidden"
        >
          {/* Tool Header */}
          <button
            onClick={() => toggleTool(invocation.toolCallId)}
            className="w-full flex items-center gap-2 px-3 py-2 hover:bg-secondary/50 transition-colors"
          >
            {/* Status Icon */}
            {invocation.state === 'pending' ? (
              <Loader2 className="h-4 w-4 text-muted-foreground animate-spin" />
            ) : invocation.state === 'result' ? (
              <CheckCircle2 className="h-4 w-4 text-green-500" />
            ) : (
              <XCircle className="h-4 w-4 text-red-500" />
            )}

            {/* Tool Icon */}
            <Wrench className="h-3.5 w-3.5 text-muted-foreground" />

            {/* Tool Name */}
            <span className="text-sm font-medium flex-1 text-left">
              {invocation.toolName}
            </span>

            {/* Expand/Collapse */}
            {expandedTools.has(invocation.toolCallId) ? (
              <ChevronUp className="h-4 w-4 text-muted-foreground" />
            ) : (
              <ChevronDown className="h-4 w-4 text-muted-foreground" />
            )}
          </button>

          {/* Expanded Content */}
          {expandedTools.has(invocation.toolCallId) && (
            <div className="border-t border-border/50 px-3 py-2 space-y-2">
              {/* Arguments */}
              <div>
                <span className="text-xs font-medium text-muted-foreground">Arguments:</span>
                <pre className="mt-1 text-xs bg-gray-800 rounded p-2 overflow-x-auto">
                  {JSON.stringify(invocation.args, null, 2)}
                </pre>
              </div>

              {/* Result */}
              {invocation.result && (
                <div>
                  <span className="text-xs font-medium text-muted-foreground">Result:</span>
                  <pre className="mt-1 text-xs bg-gray-800 rounded p-2 overflow-x-auto max-h-40 overflow-y-auto">
                    {formatToolResult(invocation.result)}
                  </pre>
                </div>
              )}
            </div>
          )}
        </div>
      ))}
    </div>
  )
}

function formatToolResult(result: string): string {
  try {
    const parsed = JSON.parse(result)
    return JSON.stringify(parsed, null, 2)
  } catch {
    return result
  }
}

interface UsageDisplayProps {
  usage: {
    inputTokens: number
    outputTokens: number
    totalTokens: number
    cost?: number
  }
}

function UsageDisplay({ usage }: UsageDisplayProps) {
  return (
    <div className="flex items-center gap-3 mt-1.5 text-xs text-muted-foreground">
      <span className="flex items-center gap-1">
        <span className="font-medium">{usage.inputTokens.toLocaleString()}</span>
        <span>in</span>
        <span className="mx-0.5">→</span>
        <span className="font-medium">{usage.outputTokens.toLocaleString()}</span>
        <span>out</span>
      </span>
      {usage.cost !== undefined && usage.cost > 0 && (
        <span className="flex items-center gap-1">
          <Coins className="h-3 w-3" />
          <span>${usage.cost.toFixed(4)}</span>
        </span>
      )}
    </div>
  )
}
