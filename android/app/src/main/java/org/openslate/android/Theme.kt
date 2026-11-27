package org.openslate.android

import androidx.compose.foundation.BorderStroke
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Shapes
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.material3.Typography
import androidx.compose.material3.lightColorScheme
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp

/** Il blu di OpenSlate: titoli, scheda scelta, pulsanti. */
private val Blue = Color(0xFF2B7BE4)

private val colors = lightColorScheme(
    primary = Blue,
    onPrimary = Color.White,
    primaryContainer = Color(0xFFDCE9FB),
    onPrimaryContainer = Color(0xFF0B3D7A),
    secondaryContainer = Color(0xFFE6EEF9),
    onSecondaryContainer = Color(0xFF1B3557),
    surface = Color.White,
    background = Color.White,
    surfaceVariant = Color(0xFFF1F4F8),
    outlineVariant = Color(0xFFDDE2E8),
)

/** Angoli morbidi, come nelle app di messaggi. */
private val shapes = Shapes(
    extraSmall = RoundedCornerShape(8.dp),
    small = RoundedCornerShape(12.dp),
    medium = RoundedCornerShape(16.dp),
    large = RoundedCornerShape(24.dp),
    extraLarge = RoundedCornerShape(28.dp),
)

/** I titoli delle schermate grandi, in grassetto e nel colore dell'app. */
private val typography = Typography().let { t ->
    t.copy(
        headlineSmall = t.headlineSmall.copy(fontWeight = FontWeight.SemiBold, color = Blue),
        titleLarge = t.titleLarge.copy(fontSize = 24.sp, fontWeight = FontWeight.SemiBold, color = Blue),
    )
}

@Composable
fun OpenSlateTheme(content: @Composable () -> Unit) {
    MaterialTheme(colorScheme = colors, shapes = shapes, typography = typography, content = content)
}

/** Una scheda della barra in basso. */
data class NavItem(val icon: ImageVector, val label: String)

/**
 * La barra in basso «flottante»: una pillola staccata dai bordi, con
 * un'ombra leggera; la scheda scelta ha la sua pillola colorata.
 */
@Composable
fun FloatingNav(items: List<NavItem>, selected: Int, onSelect: (Int) -> Unit) {
    Box(Modifier.fillMaxWidth().padding(horizontal = 14.dp, vertical = 8.dp)) {
        Surface(
            shape = RoundedCornerShape(percent = 50),
            color = MaterialTheme.colorScheme.surface,
            shadowElevation = 10.dp,
            border = BorderStroke(0.5.dp, MaterialTheme.colorScheme.outlineVariant),
        ) {
            Row(Modifier.fillMaxWidth().padding(5.dp), horizontalArrangement = Arrangement.spacedBy(2.dp)) {
                items.forEachIndexed { i, item ->
                    val on = i == selected
                    val tint = if (on) MaterialTheme.colorScheme.primary else MaterialTheme.colorScheme.onSurfaceVariant
                    Column(
                        Modifier
                            .weight(1f)
                            .clip(RoundedCornerShape(percent = 50))
                            .background(if (on) MaterialTheme.colorScheme.primaryContainer else Color.Transparent)
                            .clickable(role = Role.Tab) { onSelect(i) }
                            .padding(vertical = 6.dp),
                        horizontalAlignment = Alignment.CenterHorizontally,
                    ) {
                        Icon(item.icon, null, tint = tint)
                        Text(
                            item.label,
                            style = MaterialTheme.typography.labelMedium,
                            fontWeight = if (on) FontWeight.SemiBold else FontWeight.Normal,
                            color = tint,
                            maxLines = 1,
                            overflow = TextOverflow.Ellipsis,
                        )
                    }
                }
            }
        }
    }
}
