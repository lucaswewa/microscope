<script setup lang="ts">
// Rich text from a Thing's panel (ADR-0024): its spans drawn as elements,
// never as HTML. Only http and https links become links, and they open
// outside the app.
import { h, type VNodeChild } from 'vue'

import { useHost } from '@/host'

import type { RichText } from './types'

const props = defineProps<{ text: RichText }>()
const host = useHost()

const WEB_LINK = /^https?:\/\/[^/?#\s]+[^\s]*$/i

function Spans() {
  return props.text.map(({ text, bold, italic, href }) => {
    let node: VNodeChild = text
    if (italic) node = h('em', node)
    if (bold) node = h('strong', node)
    if (href && WEB_LINK.test(href)) {
      const open = (event: MouseEvent) => {
        event.preventDefault()
        host.openLink(href)
      }
      node = h('a', { href, onClick: open }, [node])
    }
    return node
  })
}
</script>

<template>
  <Spans />
</template>
