<template>
    <div class="metrics-chart">
        <div v-if="title" class="chart-title">{{ title }}</div>
        <canvas ref="chartCanvas"></canvas>
    </div>
</template>

<script setup lang="ts">
import { ref, onMounted, watch } from 'vue'
import type { MetricDataPoint } from '@/types'

interface Props {
    data: MetricDataPoint[]
    title?: string
    type?: 'line' | 'bar' | 'area'
    color?: string
}

const props = withDefaults(defineProps<Props>(), {
    title: '',
    type: 'line',
    color: '#3b82f6',
})

const chartCanvas = ref<HTMLCanvasElement>()

// Simple chart rendering using canvas
const renderChart = () => {
    if (!chartCanvas.value || props.data.length === 0) return

    const canvas = chartCanvas.value
    const ctx = canvas.getContext('2d')
    if (!ctx) return

    // Set canvas size
    const width = canvas.offsetWidth
    const height = canvas.offsetHeight || 200
    canvas.width = width
    canvas.height = height

    // Clear canvas
    ctx.clearRect(0, 0, width, height)

    // Calculate scales
    const padding = 40
    const chartWidth = width - padding * 2
    const chartHeight = height - padding * 2

    const values = props.data.map((d) => d.value)
    const minValue = Math.min(...values, 0)
    const maxValue = Math.max(...values)
    const valueRange = maxValue - minValue || 1

    const xStep = chartWidth / (props.data.length - 1 || 1)

    // Draw axes
    ctx.strokeStyle = '#e5e7eb'
    ctx.lineWidth = 1

    // Y-axis
    ctx.beginPath()
    ctx.moveTo(padding, padding)
    ctx.lineTo(padding, height - padding)
    ctx.stroke()

    // X-axis
    ctx.beginPath()
    ctx.moveTo(padding, height - padding)
    ctx.lineTo(width - padding, height - padding)
    ctx.stroke()

    // Draw grid lines
    ctx.strokeStyle = '#f3f4f6'
    for (let i = 0; i <= 5; i++) {
        const y = padding + (chartHeight / 5) * i
        ctx.beginPath()
        ctx.moveTo(padding, y)
        ctx.lineTo(width - padding, y)
        ctx.stroke()
    }

    // Draw data
    if (props.type === 'area') {
        // Fill area under line
        ctx.fillStyle = props.color + '20'
        ctx.beginPath()
        ctx.moveTo(padding, height - padding)
        props.data.forEach((point, i) => {
            const x = padding + i * xStep
            const y = height - padding - ((point.value - minValue) / valueRange) * chartHeight
            ctx.lineTo(x, y)
        })
        ctx.lineTo(width - padding, height - padding)
        ctx.closePath()
        ctx.fill()
    }

    if (props.type === 'line' || props.type === 'area') {
        // Draw line
        ctx.strokeStyle = props.color
        ctx.lineWidth = 2
        ctx.beginPath()
        props.data.forEach((point, i) => {
            const x = padding + i * xStep
            const y = height - padding - ((point.value - minValue) / valueRange) * chartHeight
            if (i === 0) {
                ctx.moveTo(x, y)
            } else {
                ctx.lineTo(x, y)
            }
        })
        ctx.stroke()

        // Draw points
        ctx.fillStyle = props.color
        props.data.forEach((point, i) => {
            const x = padding + i * xStep
            const y = height - padding - ((point.value - minValue) / valueRange) * chartHeight
            ctx.beginPath()
            ctx.arc(x, y, 3, 0, Math.PI * 2)
            ctx.fill()
        })
    } else if (props.type === 'bar') {
        // Draw bars
        const barWidth = Math.max(2, xStep * 0.8)
        ctx.fillStyle = props.color
        props.data.forEach((point, i) => {
            const x = padding + i * xStep - barWidth / 2
            const barHeight = ((point.value - minValue) / valueRange) * chartHeight
            const y = height - padding - barHeight
            ctx.fillRect(x, y, barWidth, barHeight)
        })
    }

    // Draw labels
    ctx.fillStyle = '#6b7280'
    ctx.font = '10px sans-serif'
    ctx.textAlign = 'right'

    // Y-axis labels
    for (let i = 0; i <= 5; i++) {
        const value = maxValue - (valueRange / 5) * i
        const y = padding + (chartHeight / 5) * i
        ctx.fillText(value.toFixed(1), padding - 5, y + 3)
    }
}

onMounted(() => {
    renderChart()
})

watch(() => props.data, renderChart, { deep: true })
</script>

<style scoped>
.metrics-chart {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
}

.chart-title {
    font-size: 0.875rem;
    font-weight: 600;
    color: var(--color-text);
}

canvas {
    width: 100%;
    height: 200px;
    border: 1px solid var(--color-border);
    border-radius: 0.375rem;
    background: var(--color-bg-secondary);
}
</style>
