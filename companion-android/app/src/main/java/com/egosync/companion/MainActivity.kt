package com.egosync.companion

import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.activity.enableEdgeToEdge
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Scaffold
import androidx.compose.material3.SnackbarHost
import androidx.compose.material3.SnackbarHostState
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.runtime.remember
import androidx.compose.ui.Modifier
import com.egosync.companion.ui.AppNavHost
import com.egosync.companion.ui.theme.EgoSyncTheme

class MainActivity : ComponentActivity() {

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        enableEdgeToEdge()
        val container = AppModelContainer.get(applicationContext)
        setContent {
            EgoSyncRoot(container)
        }
    }
}

@Composable
private fun EgoSyncRoot(container: AppModelContainer) {
    val themeMode by container.themeMode.collectAsState()
    EgoSyncTheme(themeMode = themeMode) {
        val snackbarHostState = remember { SnackbarHostState() }

        // 事件流逐条消费（SharedFlow 不去重——连续相同错误各显示一次）
        LaunchedEffect(Unit) {
            container.events.collect { message ->
                snackbarHostState.showSnackbar(message)
            }
        }

        Scaffold(
            snackbarHost = { SnackbarHost(snackbarHostState) },
            containerColor = MaterialTheme.colorScheme.background,
        ) { padding ->
            Box(
                Modifier
                    .fillMaxSize()
                    .padding(padding)
            ) {
                AppNavHost(container = container)
            }
        }
    }
}
